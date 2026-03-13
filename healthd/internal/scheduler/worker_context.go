package scheduler

import "context"

type WorkerContext struct {
	main context.Context
	sub  context.Context
}

func NewWorkerContext(main, sub context.Context) WorkerContext {
	return WorkerContext{
		main: main,
		sub:  sub,
	}
}

func (wc WorkerContext) IsDone() bool {
	if wc.main == nil || wc.sub == nil {
		return true
	}

	select {
	case <-wc.main.Done():
		return true
	default:
	}

	select {
	case <-wc.sub.Done():
		return true
	default:
		return false
	}
}
