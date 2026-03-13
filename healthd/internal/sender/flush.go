package sender

import (
	"context"
	"time"
)

type FlushCmd int

const (
	FlushArm  FlushCmd = iota // slot 非空：启动/重置 4s
	FlushStop                 // slot 清空：停 timer
)

func startFlusher(ctx context.Context, cmd <-chan FlushCmd, flushNow chan<- struct{}, after time.Duration) {
	go func() {
		var (
			timer  *time.Timer
			timerC <-chan time.Time
		)

		stopTimer := func() {
			if timer == nil {
				timerC = nil
				return
			}
			if !timer.Stop() {
				select {
				case <-timer.C:
				default:
				}
			}
			timerC = nil
		}

		resetTimer := func() {
			if timer == nil {
				timer = time.NewTimer(after)
			} else {
				timer.Reset(after)
			}
			timerC = timer.C
		}

		for {
			select {
			case <-ctx.Done():
				stopTimer()
				return

			case c := <-cmd:
				switch c {
				case FlushArm:
					resetTimer()
				case FlushStop:
					stopTimer()
				}
			case <-timerC:
				// 该 flush 了）
				select {
				case flushNow <- struct{}{}:
				default:
				}
				// 这里不自动 reset，sender 收到后 flush 并决定是否重新 arm
				stopTimer()
			}
		}
	}()
}
