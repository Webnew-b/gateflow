package probe

import (
	"context"
	"healthd/internal/app_error"
	applog "healthd/internal/log"
	"net/http"
	"strconv"
	"time"
)

type Worker struct {
	task      chan Task
	result    chan<- HealthReport
	client    *http.Client
	userAgent string
}

func NewWorker(res chan<- HealthReport, client *http.Client, userAgent string) *Worker {
	if userAgent == "" {
		userAgent = "healthd/1.0"
	}
	return &Worker{
		task:      make(chan Task, 8),
		result:    res,
		client:    client,
		userAgent: userAgent,
	}
}

func (w *Worker) StartWorker(ctx context.Context) {
	go func() {
		for {
			select {
			case <-ctx.Done():
				return
			case t, ok := <-w.task:
				if !ok {
					return
				}
				res, err := w.probeService(ctx, t)
				if err != nil {
					applog.Print(err)
					continue
				}
				select {
				case w.result <- res:
					continue
				default:
					applog.Printf("drop report app=%s name=%s isok=%t \n", res.AppUUID, res.Name, res.OK)
				case <-ctx.Done():
					return
				}
				continue
			}
		}
	}()
}

func (w *Worker) AddTask(t Task) bool {
	select {
	case w.task <- t:
		return true
	default:
		return false
	}
}

func (w *Worker) probeService(ctx context.Context, task Task) (HealthReport, error) {
	start := time.Now()
	req, err := http.NewRequestWithContext(ctx, "GET", task.Target.HealthURL, nil)
	if err != nil {
		return HealthReport{}, app_error.Wrap(app_error.CodeHTTPFailed, err, "Could not construct the http request.")
	}
	req.Header.Set("User-Agent", w.userAgent)
	resp, err := w.client.Do(req)
	lms := time.Since(start).Milliseconds()
	if err != nil || resp == nil {
		return w.getErrorHealthReport(lms, task), nil
	}
	defer resp.Body.Close()
	hr := w.getHealthdReport(resp, lms, task)

	return hr, nil
}

func (w *Worker) getHealthdReport(resp *http.Response, lms int64, task Task) HealthReport {
	expectStatus, err := strconv.Atoi(task.Target.ExpectStatus)
	ok := err == nil && resp.StatusCode == expectStatus
	errKind := ""
	if err != nil {
		errKind = "Invalid Expect Status"
	} else if !ok {
		errKind = resp.Status
	}

	return HealthReport{
		TickID:     task.TickID,
		AppUUID:    task.Target.AppUUID,
		Name:       task.Target.Name,
		CheckedAt:  task.CheckedAt,
		OK:         ok,
		ErrKind:    errKind,
		StatusCode: resp.StatusCode,
		LatencyMS:  lms,
	}
}

func (w *Worker) getErrorHealthReport(lms int64, task Task) HealthReport {

	return HealthReport{
		TickID:     task.TickID,
		AppUUID:    task.Target.AppUUID,
		Name:       task.Target.Name,
		CheckedAt:  task.CheckedAt,
		OK:         false,
		ErrKind:    "Request Fail",
		StatusCode: 0,
		LatencyMS:  lms,
	}
}
