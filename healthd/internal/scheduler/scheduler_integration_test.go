package scheduler

import (
	"context"
	"errors"
	"io"
	"net"
	"net/http"
	"strings"
	"sync"
	"testing"
	"time"

	"healthd/internal/config"
	"healthd/internal/probe"
	"healthd/internal/targets"
	gateflowv1 "healthd/v1"

	"google.golang.org/grpc"
)

type fakeGateflowClient struct {
	nodeList func(context.Context, *gateflowv1.NodeRequest) (*gateflowv1.NodeResponse, error)
}

func (f *fakeGateflowClient) Login(context.Context, *gateflowv1.LoginRequest, ...grpc.CallOption) (*gateflowv1.LoginResponse, error) {
	return nil, errors.New("unused")
}
func (f *fakeGateflowClient) AddApp(context.Context, *gateflowv1.AddAppRequest, ...grpc.CallOption) (*gateflowv1.AddAppResponse, error) {
	return nil, errors.New("unused")
}
func (f *fakeGateflowClient) ApproveApp(context.Context, *gateflowv1.ApproveAppRequest, ...grpc.CallOption) (*gateflowv1.ApproveAppResponse, error) {
	return nil, errors.New("unused")
}
func (f *fakeGateflowClient) DisableApp(context.Context, *gateflowv1.DisableAppRequest, ...grpc.CallOption) (*gateflowv1.DisableAppResponse, error) {
	return nil, errors.New("unused")
}
func (f *fakeGateflowClient) RouteUpdate(context.Context, *gateflowv1.RouteUpdateRequest, ...grpc.CallOption) (*gateflowv1.RouteUpdateResponse, error) {
	return nil, errors.New("unused")
}
func (f *fakeGateflowClient) List(context.Context, *gateflowv1.ListRequest, ...grpc.CallOption) (*gateflowv1.ListResponse, error) {
	return nil, errors.New("unused")
}
func (f *fakeGateflowClient) Show(context.Context, *gateflowv1.ShowRequest, ...grpc.CallOption) (*gateflowv1.ShowResponse, error) {
	return nil, errors.New("unused")
}
func (f *fakeGateflowClient) NodeList(ctx context.Context, req *gateflowv1.NodeRequest, _ ...grpc.CallOption) (*gateflowv1.NodeResponse, error) {
	return f.nodeList(ctx, req)
}

type roundTripFunc func(*http.Request) (*http.Response, error)

func (f roundTripFunc) RoundTrip(req *http.Request) (*http.Response, error) {
	return f(req)
}

func TestScheduler_IntegrationSyncAndCheck(t *testing.T) {
	var rpcCalls int
	fakeRPC := &fakeGateflowClient{}
	fakeRPC.nodeList = func(context.Context, *gateflowv1.NodeRequest) (*gateflowv1.NodeResponse, error) {
		rpcCalls++
		if rpcCalls <= 1 {
			return &gateflowv1.NodeResponse{List: []*gateflowv1.NodeList{{
				AppId:        "app-a",
				AppName:      "svc-a",
				HealthUrl:    "http://app-a/health",
				ExpectStatus: "200",
			}}}, nil
		}
		return &gateflowv1.NodeResponse{List: []*gateflowv1.NodeList{{
			AppId:        "app-b",
			AppName:      "svc-b",
			HealthUrl:    "http://app-b/health",
			ExpectStatus: "200",
		}}}, nil
	}

	rt := roundTripFunc(func(req *http.Request) (*http.Response, error) {
		if req.URL.String() == "http://app-a/health" || req.URL.String() == "http://app-b/health" {
			return &http.Response{
				StatusCode: http.StatusOK,
				Status:     "200 OK",
				Body:       io.NopCloser(strings.NewReader("ok")),
			}, nil
		}
		return nil, errors.New("unknown target")
	})

	ctx, cancel := context.WithTimeout(context.Background(), 800*time.Millisecond)
	defer cancel()

	cfg := config.RuntimeConfig{
		CheckInterval: 40 * time.Millisecond,
		SyncInterval:  120 * time.Millisecond,
		HTTPTimeout:   50 * time.Millisecond,
		HTTPUserAgent: "healthd-test",
		MaxInFlight:   4,
	}

	apps := map[string]bool{}
	var mu sync.Mutex

	s := Scheduler{
		context:       ctx,
		worker:        []*probe.Worker{},
		config:        cfg,
		client:        &http.Client{Timeout: cfg.HTTPTimeout, Transport: rt},
		resultChannel: make(chan probe.HealthReport, 1024),
		rpcClient:     fakeRPC,
		nodeList:      []targets.Target{},
		startSenderFn: func(ctx context.Context, receiver <-chan probe.HealthReport, _ *net.UDPConn, _ config.RuntimeConfig) {
			go func() {
				for {
					select {
					case <-ctx.Done():
						return
					case r, ok := <-receiver:
						if !ok {
							return
						}
						mu.Lock()
						apps[r.AppUUID] = true
						mu.Unlock()
					}
				}
			}()
		},
	}

	errCh := make(chan error, 1)
	go func() {
		errCh <- s.StartScheduler()
	}()

	select {
	case err := <-errCh:
		if err != nil {
			t.Fatalf("StartScheduler() err = %v", err)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("scheduler did not stop in time")
	}

	mu.Lock()
	defer mu.Unlock()
	if !apps["app-a"] {
		t.Fatalf("expected reports for app-a, got %v", apps)
	}
	if !apps["app-b"] {
		t.Fatalf("expected reports for app-b after sync, got %v", apps)
	}
}
