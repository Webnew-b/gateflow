package probe

import (
	"context"
	"errors"
	"io"
	"net/http"
	"strings"
	"testing"
	"time"

	"healthd/internal/targets"
)

type roundTripFunc func(*http.Request) (*http.Response, error)

func (f roundTripFunc) RoundTrip(req *http.Request) (*http.Response, error) {
	return f(req)
}

func testTask(url, expect string) Task {
	return Task{
		TickID:    1,
		CheckedAt: time.Now().UnixMilli(),
		Target: targets.Target{
			AppUUID:      "app-1",
			Name:         "svc-1",
			HealthURL:    url,
			ExpectStatus: expect,
		},
	}
}

func clientWithRT(rt http.RoundTripper, timeout time.Duration) *http.Client {
	return &http.Client{Transport: rt, Timeout: timeout}
}

func TestProbeService_HealthyStatus(t *testing.T) {
	for _, status := range []int{http.StatusOK, http.StatusNoContent} {
		t.Run(http.StatusText(status), func(t *testing.T) {
			rt := roundTripFunc(func(req *http.Request) (*http.Response, error) {
				_ = req
				return &http.Response{
					StatusCode: status,
					Status:     http.StatusText(status),
					Body:       io.NopCloser(strings.NewReader("ok")),
				}, nil
			})
			w := NewWorker(make(chan HealthReport, 1), clientWithRT(rt, 200*time.Millisecond), "test-agent")
			h, err := w.probeService(context.Background(), testTask("http://example/health", strconvStatus(status)))
			if err != nil {
				t.Fatalf("probeService() err = %v", err)
			}
			if !h.OK {
				t.Fatalf("report OK = false, want true: %+v", h)
			}
			if h.StatusCode != status {
				t.Fatalf("status = %d, want %d", h.StatusCode, status)
			}
			if h.LatencyMS < 0 {
				t.Fatalf("latency should be >= 0, got %d", h.LatencyMS)
			}
		})
	}
}

func TestProbeService_UnhealthyAndTimeout(t *testing.T) {
	t.Run("server returns 500", func(t *testing.T) {
		rt := roundTripFunc(func(req *http.Request) (*http.Response, error) {
			_ = req
			return &http.Response{
				StatusCode: http.StatusInternalServerError,
				Status:     "500 Internal Server Error",
				Body:       io.NopCloser(strings.NewReader("boom")),
			}, nil
		})

		w := NewWorker(make(chan HealthReport, 1), clientWithRT(rt, 200*time.Millisecond), "test-agent")
		h, err := w.probeService(context.Background(), testTask("http://example/health", "200"))
		if err != nil {
			t.Fatalf("probeService() err = %v", err)
		}
		if h.OK {
			t.Fatalf("report OK = true, want false: %+v", h)
		}
		if h.StatusCode != http.StatusInternalServerError {
			t.Fatalf("status = %d, want 500", h.StatusCode)
		}
		if h.ErrKind == "" {
			t.Fatalf("ErrKind should not be empty")
		}
	})

	t.Run("timeout controlled by context/client timeout", func(t *testing.T) {
		rt := roundTripFunc(func(req *http.Request) (*http.Response, error) {
			<-req.Context().Done()
			return nil, req.Context().Err()
		})

		w := NewWorker(make(chan HealthReport, 1), clientWithRT(rt, 50*time.Millisecond), "test-agent")
		start := time.Now()
		h, err := w.probeService(context.Background(), testTask("http://example/slow", "200"))
		elapsed := time.Since(start)
		if err != nil {
			t.Fatalf("probeService() err = %v", err)
		}
		if h.OK {
			t.Fatalf("report OK = true, want false: %+v", h)
		}
		if h.StatusCode != 0 {
			t.Fatalf("status = %d, want 0", h.StatusCode)
		}
		if elapsed > 250*time.Millisecond {
			t.Fatalf("timeout not respected, elapsed=%v", elapsed)
		}
	})

	t.Run("unreachable host style error", func(t *testing.T) {
		rt := roundTripFunc(func(req *http.Request) (*http.Response, error) {
			_ = req
			return nil, errors.New("connect: connection refused")
		})
		w := NewWorker(make(chan HealthReport, 1), clientWithRT(rt, 100*time.Millisecond), "test-agent")
		h, err := w.probeService(context.Background(), testTask("http://127.0.0.1:1", "200"))
		if err != nil {
			t.Fatalf("probeService() err = %v", err)
		}
		if h.OK {
			t.Fatalf("report OK = true, want false: %+v", h)
		}
		if h.StatusCode != 0 {
			t.Fatalf("status = %d, want 0", h.StatusCode)
		}
	})

	t.Run("invalid expect status", func(t *testing.T) {
		rt := roundTripFunc(func(req *http.Request) (*http.Response, error) {
			_ = req
			return &http.Response{
				StatusCode: http.StatusOK,
				Status:     "200 OK",
				Body:       io.NopCloser(strings.NewReader("ok")),
			}, nil
		})
		w := NewWorker(make(chan HealthReport, 1), clientWithRT(rt, 200*time.Millisecond), "test-agent")
		h, err := w.probeService(context.Background(), testTask("http://example/health", "abc"))
		if err != nil {
			t.Fatalf("probeService() err = %v", err)
		}
		if h.OK {
			t.Fatalf("report OK = true, want false: %+v", h)
		}
		if h.ErrKind != "Invalid Expect Status" {
			t.Fatalf("ErrKind = %q, want %q", h.ErrKind, "Invalid Expect Status")
		}
	})
}

func strconvStatus(v int) string {
	if v == http.StatusOK {
		return "200"
	}
	if v == http.StatusNoContent {
		return "204"
	}
	return "200"
}
