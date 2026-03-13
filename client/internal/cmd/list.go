package cmd

import (
	"client/internal/tools"
	gateflowv1 "client/v1"
	"context"
	"sort"
	"strings"
	"unicode/utf8"
)

func ListApps(client gateflowv1.GateflowServiceClient, token string, context context.Context) (string, error) {
	// 1) auth
	context = AddAuth(context, token)

	// 2) rpc
	fRes, err := client.List(context, &gateflowv1.ListRequest{})
	if err != nil {
		return "", tools.AppError{
			Code:    tools.InvalidRequest,
			Message: "Could not ListApps",
			Err:     err,
		}
	}

	rows := fRes.GetList()

	// 可选：排序让输出稳定
	sort.Slice(rows, func(i, j int) bool {
		return rows[i].GetAppId() < rows[j].GetAppId()
	})

	// 3) table
	headers := []string{
		"APP ID",
		"APP NAME",
		"STATUS",
		"MOUNT PATH",
		"UPSTREAM PATH",
		"TARGET URL",
	}

	widths := make([]int, len(headers))
	for i, h := range headers {
		widths[i] = runeLen(h)
	}

	// 扫描字段宽度（全部字段）
	for _, r := range rows {
		fields := []string{
			r.GetAppId(),
			r.GetAppName(),
			r.GetStatus(),
			r.GetMountPath(),
			r.GetUpstreamPath(),
			r.GetTargetUrl(),
		}
		for i := range fields {
			if l := runeLen(fields[i]); l > widths[i] {
				widths[i] = l
			}
		}
	}

	var b strings.Builder
	sep := "  "

	writeRow(&b, headers, widths, sep)
	b.WriteByte('\n')

	total := 0
	for i, w := range widths {
		total += w
		if i != 0 {
			total += len(sep)
		}
	}
	b.WriteString(strings.Repeat("-", total))
	b.WriteByte('\n')

	for _, r := range rows {
		fields := []string{
			r.GetAppId(),
			r.GetAppName(),
			r.GetStatus(),
			r.GetMountPath(),
			r.GetUpstreamPath(),
			r.GetTargetUrl(),
		}
		writeRow(&b, fields, widths, sep)
		b.WriteByte('\n')
	}

	return b.String(), nil
}

func writeRow(b *strings.Builder, cols []string, widths []int, sep string) {
	for i, v := range cols {
		if i > 0 {
			b.WriteString(sep)
		}
		b.WriteString(padRight(v, widths[i]))
	}
}

func padRight(s string, width int) string {
	cur := runeLen(s)
	if cur >= width {
		return s
	}
	return s + strings.Repeat(" ", width-cur)
}

func runeLen(s string) int {
	return utf8.RuneCountInString(s)
}
