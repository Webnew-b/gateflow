package scheduler

import (
	"context"
	"healthd/internal/config"
	applog "healthd/internal/log"
	"healthd/internal/probe"
	"healthd/internal/sender"
	"healthd/internal/targets"
	gateflowv1 "healthd/v1"
	"net"
	"net/http"
	"sync"
	"sync/atomic"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
)

const targetSyncRPCTimeout = 5 * time.Second

type Scheduler struct {
	context       context.Context
	worker        []*probe.Worker
	config        config.RuntimeConfig
	client        *http.Client
	resultChannel chan probe.HealthReport
	conn          *net.UDPConn
	rpcClient     gateflowv1.GateflowServiceClient
	rpcConn       *grpc.ClientConn
	nodeList      []targets.Target
	tickID        uint64
	targetsMu     sync.RWMutex
	checkBusy     atomic.Bool
	checkRunner   func()
	startSenderFn func(context.Context, <-chan probe.HealthReport, *net.UDPConn, config.RuntimeConfig)
}

func (s *Scheduler) startWorker() {
	var workers []*probe.Worker
	for range s.config.MaxInFlight {
		worker := probe.NewWorker(s.resultChannel, s.client, s.config.HTTPUserAgent)
		worker.StartWorker(s.context)
		workers = append(workers, worker)
	}
	s.worker = workers
}

func (s *Scheduler) startSender() {
	if s.startSenderFn != nil {
		s.startSenderFn(s.context, s.resultChannel, s.conn, s.config)
		return
	}
	sender.StartSender(s.context, s.resultChannel, s.conn, s.config)
}

func (s *Scheduler) getTargetConfig() error {
	baseCtx, cancel := context.WithTimeout(s.context, targetSyncRPCTimeout)
	defer cancel()

	rpcCtx := metadata.NewOutgoingContext(
		baseCtx,
		metadata.Pairs("authorization", "Bearer "+s.config.GatewaySessionToken),
	)
	res, err := s.rpcClient.NodeList(rpcCtx, &gateflowv1.NodeRequest{})
	if err != nil {
		return err
	}
	s.targetsMu.Lock()
	s.nodeList = targets.ParseTargets(res.List)
	s.targetsMu.Unlock()
	return nil
}

func (s *Scheduler) Close() {
	s.client = nil
	s.rpcClient = nil
	if s.conn != nil {
		_ = s.conn.Close()
		s.conn = nil
	}
	if s.rpcConn != nil {
		_ = s.rpcConn.Close()
	}
	s.rpcConn = nil
}

func (s *Scheduler) nextTickID() uint64 {
	s.tickID++
	return s.tickID
}

func (s *Scheduler) runCheck() {
	s.targetsMu.RLock()
	targetsSnapshot := append([]targets.Target(nil), s.nodeList...)
	s.targetsMu.RUnlock()

	if len(targetsSnapshot) == 0 || len(s.worker) == 0 {
		return
	}

	tickID := s.nextTickID()
	checkedAt := time.Now().UnixMilli()
	workerCount := len(s.worker)

	for i, target := range targetsSnapshot {
		task := probe.Task{
			TickID:    tickID,
			Target:    target,
			CheckedAt: checkedAt,
		}

		start := i % workerCount
		dispatched := false
		for n := 0; n < workerCount; n++ {
			idx := (start + n) % workerCount
			if s.worker[idx].AddTask(task) {
				dispatched = true
				break
			}
		}
		if !dispatched {
			applog.Printf("drop task app=%s name=%s reason=all_worker_queue_full", target.AppUUID, target.Name)
		}
	}
}

func (s *Scheduler) StartScheduler() error {
	s.startSender()
	s.startWorker()

	// 启动后先同步一次，确保初始快照可用
	if err := s.getTargetConfig(); err != nil {
		return err
	}

	checkTicker := time.NewTicker(s.config.CheckInterval)
	syncTicker := time.NewTicker(s.config.SyncInterval)
	defer checkTicker.Stop()
	defer syncTicker.Stop()
	defer s.Close()

	return s.runEventLoop(checkTicker.C, syncTicker.C)
}

func (s *Scheduler) runEventLoop(checkTick <-chan time.Time, syncTick <-chan time.Time) error {
	for {
		select {
		case <-s.context.Done():
			return nil
		case <-syncTick:
			if err := s.getTargetConfig(); err != nil {
				applog.Printf("sync targets failed: %v", err)
			}
		case <-checkTick:
			if !s.checkBusy.CompareAndSwap(false, true) {
				applog.Println("skip check tick: previous check still running")
				continue
			}
			go func() {
				defer s.checkBusy.Store(false)
				if s.checkRunner != nil {
					s.checkRunner()
					return
				}
				s.runCheck()
			}()
		}
	}
}

func newUDPClient(remote string) (*net.UDPConn, error) {
	raddr, err := net.ResolveUDPAddr("udp", remote) // 例如 "127.0.0.1:9000"
	if err != nil {
		return nil, err
	}
	// localAddr 传 nil：让系统自动分配本地端口
	conn, err := net.DialUDP("udp", nil, raddr)
	if err != nil {
		return nil, err
	}
	return conn, nil
}

func CreateScheduler(ctx context.Context) (Scheduler, error) {
	runtimeConfig, err := config.LoadConfigFromProjectRoot()
	if err != nil {
		return Scheduler{}, err
	}

	rpcConn, err := grpc.NewClient(
		runtimeConfig.GatewayAdminRPCEndpoint,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		return Scheduler{}, err
	}

	rpcClient := gateflowv1.NewGateflowServiceClient(rpcConn)

	client := &http.Client{
		Timeout: runtimeConfig.HTTPTimeout,
	}

	conn, err := newUDPClient(runtimeConfig.GatewayUDPAddr)

	if err != nil {
		rpcConn.Close()
		return Scheduler{}, err
	}

	return Scheduler{
		context:       ctx,
		worker:        []*probe.Worker{},
		config:        runtimeConfig,
		client:        client,
		resultChannel: make(chan probe.HealthReport, 1000),
		conn:          conn,
		rpcClient:     rpcClient,
		rpcConn:       rpcConn,
		nodeList:      []targets.Target{},
		tickID:        0,
	}, nil
}
