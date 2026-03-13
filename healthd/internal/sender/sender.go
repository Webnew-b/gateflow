package sender

import (
	"context"
	"encoding/json"
	"healthd/internal/app_error"
	"healthd/internal/config"
	applog "healthd/internal/log"
	"healthd/internal/probe"
	"healthd/internal/targets"
	"io"
	"net"
	"time"
)

func CreateUDPClient(url string) (*net.UDPConn, error) {
	addr, err := net.ResolveUDPAddr("udp", url)
	if err != nil {
		return nil, err
	}
	conn, err := net.DialUDP("udp", nil, addr)
	if err != nil {
		return nil, err
	}
	return conn, nil
}

func StartSender(ctx context.Context, receiver <-chan probe.HealthReport, conn *net.UDPConn, config config.RuntimeConfig) {
	startSenderWithWriter(ctx, receiver, conn, (config.HTTPTimeout+1)*time.Second)
}

func startSenderWithWriter(ctx context.Context, receiver <-chan probe.HealthReport, writer io.Writer, flushAfter time.Duration) {
	flushCmd := make(chan FlushCmd, 8)
	flushNow := make(chan struct{}, 1)
	startFlusher(ctx, flushCmd, flushNow, flushAfter)

	go func() {
		var hrSlot targets.Slot[probe.HealthReport]
		var curTick uint64

		flush := func() {
			if hrSlot.IsEmpty() {
				return
			}
			err := sendReport(hrSlot, writer)
			if err != nil {
				applog.Println(err)
			}
			hrSlot = targets.Slot[probe.HealthReport]{}
			curTick = 0
			// 清空了，告诉 flusher 停 timer
			select {
			case flushCmd <- FlushStop:
			default:
			}
		}

		onAdded := func() {
			// slot 非空，重置 4s
			select {
			case flushCmd <- FlushArm:
			default:
			}
		}
		for {
			select {
			case <-ctx.Done():
				flush()
				return

			case <-flushNow:
				// 4s 到：不足 8 也 flush
				flush()

			case res, ok := <-receiver:
				if !ok {
					flush()
					return
				}

				// 空 slot，初始化 tick、加入、arm
				if hrSlot.IsEmpty() {
					curTick = res.TickID
					_, _ = hrSlot.TryAdd(res)
					onAdded()
					// 满了就 flush
					if hrSlot.IsFull() {
						flush()
					}
					continue
				}

				// TickID 变化，先 flush，再加入新 slot
				if res.TickID != curTick {
					flush()
					curTick = res.TickID
					_, _ = hrSlot.TryAdd(res)
					onAdded()
					continue
				}

				// Tick 相同，尝试加入，满了就 flush 再加入
				if _, ok := hrSlot.TryAdd(res); !ok {
					flush()
					curTick = res.TickID
					_, _ = hrSlot.TryAdd(res)
					onAdded()
					continue
				}

				onAdded()

				if hrSlot.IsFull() {
					flush()
				}
			}
		}
	}()
}

func sendReport(rp targets.Slot[probe.HealthReport], writer io.Writer) error {
	if rp.IsEmpty() {
		return nil
	}

	reportList := rp.Unpack()
	reportJson := probe.ReportBatch{
		TickID:    reportList[0].TickID,
		CheckedAt: reportList[0].CheckedAt,
		Reports:   reportList,
	}
	reportReq, err := json.Marshal(reportJson)
	if err != nil {
		return app_error.Wrap(app_error.CodeEncodeFailed, err, "Could not marshal report.")
	}

	_, err = writer.Write(reportReq)
	if err != nil {
		return app_error.Wrap(app_error.CodeUDPFailed, err, "Could not send report")
	}
	return nil
}
