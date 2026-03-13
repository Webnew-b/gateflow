package main

import (
	"client/internal/cmd"
	gateflowv1 "client/v1"
	"context"
	"fmt"
	"os"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

func main() {
	// 解析命令
	parsed := cmd.ParseCommand(os.Args[1:])
	if parsed.Mode == "" {
		fmt.Fprintln(os.Stderr, "error: missing mode (e.g. app/route/login)")
		os.Exit(1)
	}

	//host：优先读 APP_HOST（你已经有 GetHost()）
	host, err := cmd.GetHost()
	if err != nil || host == "" {
		host = "127.0.0.1:50051"
	}

	// 建立 gRPC 连接
	conn, err := grpc.NewClient(
		host,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	defer conn.Close()

	client := gateflowv1.NewGateflowServiceClient(conn)

	// 全局超时
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()

	// 执行命令
	if err := cmd.SelectCmd(&parsed, client, ctx); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
