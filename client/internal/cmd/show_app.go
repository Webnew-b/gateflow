package cmd

import (
	"client/internal/tools"
	gateflowv1 "client/v1"
	"context"
	"strings"
)

type ShowAppReq struct {
	AppID   string
	AppName string
}

func showAppVerify(req *ShowAppReq) error {
	if req == nil {
		return tools.AppError{
			Code:    tools.InvalidRequest,
			Message: "request is nil",
			Err:     nil,
		}
	}
	if req.AppName == "" && req.AppID == "" {
		return tools.AppError{
			Code:    tools.InvalidArg,
			Message: "either app_name or app_id is required",
			Err:     nil,
		}
	}
	if req.AppID != "" && !IsUUID(req.AppID) {
		return tools.AppError{
			Code:    tools.InvalidArg,
			Message: "app_id is invalid",
			Err:     nil,
		}
	}
	if req.AppName != "" && !IsLowerAlphaNumDash(req.AppName) {
		return tools.AppError{
			Code:    tools.InvalidArg,
			Message: "app_name is invalid",
			Err:     nil,
		}
	}
	return nil
}

func getShowIdentify(req *ShowAppReq) (*gateflowv1.ShowRequest, error) {
	if err := showAppVerify(req); err != nil {
		return &gateflowv1.ShowRequest{}, err
	}

	// 有 name 优先用 name（你也可以改成 id 优先）
	if req.AppName != "" {
		return &gateflowv1.ShowRequest{
			AppIdentify:  req.AppName,
			IdentifyType: string(AppName),
		}, nil
	}
	return &gateflowv1.ShowRequest{
		AppIdentify:  req.AppID,
		IdentifyType: string(AppId),
	}, nil
}

// ShowApp: 带 token 鉴权，返回 “字段名-字段值” 两列表格
func ShowApp(client gateflowv1.GateflowServiceClient, token string, req *ShowAppReq, ctx context.Context) (string, error) {
	ctx = AddAuth(ctx, token)

	fReq, err := getShowIdentify(req)
	if err != nil {
		return "", err
	}

	fRes, err := client.Show(ctx, fReq)
	if err != nil {
		return "", tools.AppError{
			Code:    tools.InvalidRequest,
			Message: "Could not show app.",
			Err:     err,
		}
	}

	app := fRes.App
	if app == nil {
		return "", tools.AppError{
			Code:    tools.InvalidRequest,
			Message: "app not found (empty response)",
			Err:     nil,
		}
	}

	// 组装 “名称 / 属性” 二列表
	pairs := [][2]string{
		{"app_id", app.GetAppId()},
		{"app_name", app.GetAppName()},
		{"status", app.GetStatus()},
		{"mount_path", app.GetMountPath()},
		{"upstream_path", app.GetUpstreamPath()},
		{"target_url", app.GetTargetUrl()},
	}

	return formatKVTable(pairs), nil
}

// --json 选项，也可以用这个：
// func showAppJSON(resp *gateflowv1.ShowResponse) (string, error) {
// 	b, err := protojson.MarshalOptions{UseProtoNames: true, Indent: "  "}.Marshal(resp)
// 	if err != nil { return "", err }
// 	return string(b), nil
// }

func formatKVTable(pairs [][2]string) string {
	w1, w2 := runeLen("NAME"), runeLen("VALUE")
	for _, p := range pairs {
		if l := runeLen(p[0]); l > w1 {
			w1 = l
		}
		if l := runeLen(p[1]); l > w2 {
			w2 = l
		}
	}

	var b strings.Builder
	sep := "  "

	b.WriteString(padRight("NAME", w1))
	b.WriteString(sep)
	b.WriteString(padRight("VALUE", w2))
	b.WriteByte('\n')

	b.WriteString(strings.Repeat("-", w1+runeLen(sep)+w2))
	b.WriteByte('\n')

	for _, p := range pairs {
		b.WriteString(padRight(p[0], w1))
		b.WriteString(sep)
		b.WriteString(p[1])
		b.WriteByte('\n')
	}

	return b.String()
}
