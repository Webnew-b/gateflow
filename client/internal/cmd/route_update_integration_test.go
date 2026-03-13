package cmd

import (
	"context"
	"net"
	"strings"
	"sync"
	"testing"

	gateflowv1 "client/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/test/bufconn"
)

const testBufSize = 1024 * 1024

type routeUpdateTestServer struct {
	gateflowv1.UnimplementedGateflowServiceServer

	mu      sync.Mutex
	lastReq *gateflowv1.RouteUpdateRequest
	auth    string
}

func (s *routeUpdateTestServer) RouteUpdate(ctx context.Context, req *gateflowv1.RouteUpdateRequest) (*gateflowv1.RouteUpdateResponse, error) {
	md, _ := metadata.FromIncomingContext(ctx)
	var auth string
	if values := md.Get("authorization"); len(values) > 0 {
		auth = values[0]
	}

	s.mu.Lock()
	s.lastReq = req
	s.auth = auth
	s.mu.Unlock()

	return &gateflowv1.RouteUpdateResponse{
		AppId:        "app-id-1",
		AppName:      req.AppName,
		Status:       "Updated",
		MountPath:    req.MountPath,
		UpstreamPath: req.UpstreamPath,
	}, nil
}

func TestRouteUpdate_Integration_SendsCorrectRequestAndAuth(t *testing.T) {
	ctx := context.Background()
	serverImpl := &routeUpdateTestServer{}
	client, cleanup := newBufconnClient(t, serverImpl)
	defer cleanup()

	out, err := RouteUpdate(client, "test-token", &RouteUpdateReq{
		AppName:      "demo",
		MountPath:    "/demo",
		UpstreamPath: "/api",
	}, ctx)
	if err != nil {
		t.Fatalf("RouteUpdate() error = %v, want nil", err)
	}

	serverImpl.mu.Lock()
	gotReq := serverImpl.lastReq
	gotAuth := serverImpl.auth
	serverImpl.mu.Unlock()

	if gotReq == nil {
		t.Fatal("server did not receive RouteUpdate request")
	}
	if gotReq.GetUpstreamPath() != "/api" {
		t.Fatalf("UpstreamPath = %q, want %q", gotReq.GetUpstreamPath(), "/api")
	}
	if gotAuth != "Bearer test-token" {
		t.Fatalf("authorization = %q, want %q", gotAuth, "Bearer test-token")
	}
	if !strings.Contains(out, `"upstreamPath":"/api"`) {
		t.Fatalf("output JSON missing upstreamPath, got: %s", out)
	}
}

func newBufconnClient(t *testing.T, impl gateflowv1.GateflowServiceServer) (gateflowv1.GateflowServiceClient, func()) {
	t.Helper()

	lis := bufconn.Listen(testBufSize)
	srv := grpc.NewServer()
	gateflowv1.RegisterGateflowServiceServer(srv, impl)

	go func() {
		_ = srv.Serve(lis)
	}()

	conn, err := grpc.DialContext(
		context.Background(),
		"bufnet",
		grpc.WithContextDialer(func(context.Context, string) (net.Conn, error) {
			return lis.Dial()
		}),
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		srv.Stop()
		_ = lis.Close()
		t.Fatalf("grpc dial failed: %v", err)
	}

	cleanup := func() {
		_ = conn.Close()
		srv.Stop()
		_ = lis.Close()
	}
	return gateflowv1.NewGateflowServiceClient(conn), cleanup
}
