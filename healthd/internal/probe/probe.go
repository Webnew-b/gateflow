package probe

import (
	"context"
	"healthd/internal/targets"
	"time"
)

type Task struct {
	TickID    uint64
	Target    targets.Target
	CheckedAt int64
}

type HealthReport struct {
	TickID     uint64 `json:"-"`
	AppUUID    string `json:"app_uuid"`
	Name       string `json:"name"`
	CheckedAt  int64  `json:"checked_at"`
	OK         bool   `json:"ok"`
	ErrKind    string `json:"err_kind,omitempty"`
	StatusCode int    `json:"status_code"`
	LatencyMS  int64  `json:"latency_ms"`
}

type ReportBatch struct {
	TickID    uint64         `json:"tick_id"`
	CheckedAt int64          `json:"checked_at"`
	Reports   []HealthReport `json:"reports"`
}

type TickReq struct {
	Reply chan uint64
}

func StartTickActor(ctx context.Context) chan<- TickReq {
	inbox := make(chan TickReq, 64)

	go func() {
		var seq uint64
		for {
			select {
			case <-ctx.Done():
				return
			case req := <-inbox:
				seq++
				select {
				case req.Reply <- seq:
				case <-ctx.Done():
					return
				}
			}
		}
	}()
	return inbox
}

func NextTickID(ctx context.Context, inbox chan<- TickReq) (uint64, bool) {
	reply := make(chan uint64, 1)
	select {
	case inbox <- TickReq{reply}:
	case <-ctx.Done():
		return 0, false
	}

	select {
	case id := <-reply:
		return id, true
	case <-ctx.Done():
		return 0, false
	}
}

func targetToTaskSlots(tickID uint64, ts []targets.Slot[targets.Target]) []targets.Slot[Task] {
	var taskSlots []targets.Slot[Task]
	for _, slot := range ts {
		res := targets.MapSlot(slot, func(t targets.Target) Task {
			return Task{
				TickID:    tickID,
				Target:    t,
				CheckedAt: time.Now().UnixMilli(),
			}
		})
		taskSlots = append(taskSlots, res)
	}
	return taskSlots
}
