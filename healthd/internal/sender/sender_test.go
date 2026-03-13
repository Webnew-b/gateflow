package sender

import (
	"bytes"
	"context"
	"encoding/json"
	"strings"
	"sync"
	"testing"
	"time"

	"healthd/internal/probe"
	"healthd/internal/targets"
)

type recordingWriter struct {
	mu   sync.Mutex
	data [][]byte
}

func (w *recordingWriter) Write(p []byte) (int, error) {
	w.mu.Lock()
	defer w.mu.Unlock()
	cp := make([]byte, len(p))
	copy(cp, p)
	w.data = append(w.data, cp)
	return len(p), nil
}

func (w *recordingWriter) count() int {
	w.mu.Lock()
	defer w.mu.Unlock()
	return len(w.data)
}

func (w *recordingWriter) lastBatch(t *testing.T) probe.ReportBatch {
	t.Helper()
	w.mu.Lock()
	defer w.mu.Unlock()
	if len(w.data) == 0 {
		t.Fatal("no payload written")
	}
	var batch probe.ReportBatch
	if err := json.Unmarshal(w.data[len(w.data)-1], &batch); err != nil {
		t.Fatalf("Unmarshal() err = %v", err)
	}
	return batch
}

func sampleReport(tick uint64, app string) probe.HealthReport {
	return probe.HealthReport{
		TickID:     tick,
		AppUUID:    app,
		Name:       app,
		CheckedAt:  time.Now().UnixMilli(),
		OK:         true,
		StatusCode: 200,
		LatencyMS:  1,
	}
}

func TestSendReport_EncodeDecodeRoundTrip(t *testing.T) {
	var slot targets.Slot[probe.HealthReport]
	_, _ = slot.TryAdd(sampleReport(1, "a"))
	_, _ = slot.TryAdd(sampleReport(1, "b"))

	var buf bytes.Buffer
	if err := sendReport(slot, &buf); err != nil {
		t.Fatalf("sendReport() err = %v", err)
	}

	var got probe.ReportBatch
	if err := json.Unmarshal(buf.Bytes(), &got); err != nil {
		t.Fatalf("Unmarshal() err = %v", err)
	}
	if got.TickID != 1 {
		t.Fatalf("TickID = %d, want 1", got.TickID)
	}
	if len(got.Reports) != 2 {
		t.Fatalf("len(Reports) = %d, want 2", len(got.Reports))
	}
}

func TestSendReport_KeepsZeroStatusCodeField(t *testing.T) {
	var slot targets.Slot[probe.HealthReport]
	_, _ = slot.TryAdd(probe.HealthReport{
		TickID:     1,
		AppUUID:    "app-zero",
		Name:       "app-zero",
		CheckedAt:  time.Now().UnixMilli(),
		OK:         false,
		StatusCode: 0,
		LatencyMS:  2,
	})

	var buf bytes.Buffer
	if err := sendReport(slot, &buf); err != nil {
		t.Fatalf("sendReport() err = %v", err)
	}

	got := buf.String()
	if !strings.Contains(got, `"status_code":0`) {
		t.Fatalf("payload missing zero status_code field: %s", got)
	}
}

func TestStartSender_FlushOnBatchFull(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	receiver := make(chan probe.HealthReport, 16)
	writer := &recordingWriter{}
	startSenderWithWriter(ctx, receiver, writer, 5*time.Second)

	for i := 0; i < 8; i++ {
		receiver <- sampleReport(1, "app")
	}

	deadline := time.Now().Add(300 * time.Millisecond)
	for writer.count() == 0 && time.Now().Before(deadline) {
		time.Sleep(5 * time.Millisecond)
	}
	if writer.count() == 0 {
		t.Fatal("expected at least one flush")
	}
	batch := writer.lastBatch(t)
	if len(batch.Reports) != 8 {
		t.Fatalf("len(batch.Reports) = %d, want 8", len(batch.Reports))
	}
}

func TestStartSender_FlushOnTimer(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	receiver := make(chan probe.HealthReport, 4)
	writer := &recordingWriter{}
	startSenderWithWriter(ctx, receiver, writer, 80*time.Millisecond)
	receiver <- sampleReport(1, "app")

	deadline := time.Now().Add(400 * time.Millisecond)
	for writer.count() == 0 && time.Now().Before(deadline) {
		time.Sleep(5 * time.Millisecond)
	}
	if writer.count() == 0 {
		t.Fatal("expected timer-driven flush")
	}
	batch := writer.lastBatch(t)
	if len(batch.Reports) != 1 {
		t.Fatalf("len(batch.Reports) = %d, want 1", len(batch.Reports))
	}
}
