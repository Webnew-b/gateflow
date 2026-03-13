package scheduler

import (
	"context"
	"sync/atomic"
	"testing"
	"time"
)

func TestRunEventLoop_SkipIfBusy(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	s := &Scheduler{context: ctx}
	checkTick := make(chan time.Time, 8)
	syncTick := make(chan time.Time, 1)
	release := make(chan struct{})

	var runs atomic.Int32
	s.checkRunner = func() {
		runs.Add(1)
		<-release
	}

	done := make(chan struct{})
	go func() {
		_ = s.runEventLoop(checkTick, syncTick)
		close(done)
	}()

	checkTick <- time.Now()
	deadline := time.Now().Add(300 * time.Millisecond)
	for runs.Load() != 1 && time.Now().Before(deadline) {
		time.Sleep(5 * time.Millisecond)
	}
	if runs.Load() != 1 {
		t.Fatalf("runs = %d, want 1", runs.Load())
	}

	checkTick <- time.Now()
	checkTick <- time.Now()
	time.Sleep(40 * time.Millisecond)
	if runs.Load() != 1 {
		t.Fatalf("busy checks should be skipped, runs = %d, want 1", runs.Load())
	}

	close(release)
	time.Sleep(30 * time.Millisecond)
	checkTick <- time.Now()

	deadline = time.Now().Add(300 * time.Millisecond)
	for runs.Load() != 2 && time.Now().Before(deadline) {
		time.Sleep(5 * time.Millisecond)
	}
	if runs.Load() != 2 {
		t.Fatalf("runs after release = %d, want 2", runs.Load())
	}

	cancel()
	select {
	case <-done:
	case <-time.After(300 * time.Millisecond):
		t.Fatal("runEventLoop did not exit after cancel")
	}
}
