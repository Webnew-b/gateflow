package main

import (
	"context"
	"healthd/internal/config"
	applog "healthd/internal/log"
	"healthd/internal/scheduler"
	"os"
	"os/signal"
	"syscall"
)

func main() {
	if _, _, err := config.EnsureExampleConfigExists(); err != nil {
		applog.Printf("ensure config failed: %v", err)
		os.Exit(1)
	}

	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	sc, err := scheduler.CreateScheduler(ctx)
	if err != nil {
		applog.Printf("create scheduler failed: %v", err)
		os.Exit(1)
	}
	if err := sc.StartScheduler(); err != nil {
		applog.Printf("run scheduler failed: %v", err)
		os.Exit(1)
	}
}
