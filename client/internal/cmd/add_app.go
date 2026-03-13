package cmd

import (
	"client/internal/tools"
	gateflowv1 "client/v1"
	"context"
	"fmt"
	"regexp"
	"strings"

	"google.golang.org/protobuf/encoding/protojson"
)

type AddAppReq struct {
	AppName      string
	TargetURL    string
	MountPath    string
	UpstreamPath string
	Secret       *string // optional
}

func AddAppVerify(req *AddAppReq) error {
	if req == nil {
		return tools.AppErrorNew(tools.InvalidRequest, "request is nil", nil)
	}

	req.AppName = strings.TrimSpace(req.AppName)
	req.TargetURL = strings.TrimSpace(req.TargetURL)
	req.MountPath = strings.TrimSpace(req.MountPath)
	req.UpstreamPath = strings.TrimSpace(req.UpstreamPath)

	// 判空：属于请求不完整
	if req.AppName == "" {
		return tools.AppErrorNew(tools.InvalidRequest, "app_name is required", nil)
	}
	if req.TargetURL == "" {
		return tools.AppErrorNew(tools.InvalidRequest, "target_url is required", nil)
	}
	if req.MountPath == "" {
		return tools.AppErrorNew(tools.InvalidRequest, "mount_path is required", nil)
	}
	if req.UpstreamPath == "" {
		return tools.AppErrorNew(tools.InvalidRequest, "upstream_path is required", nil)
	}

	// name：小写字母/数字/-（不允许头尾-，不允许连续--)
	nameRe := regexp.MustCompile(`^[a-z0-9]+(?:-[a-z0-9]+)*$`)
	if !nameRe.MatchString(req.AppName) {
		return tools.AppErrorNew(tools.InvalidArg, fmt.Sprintf("app_name invalid: %q", req.AppName), nil)
	}

	// target_url：http/https + host
	if err := ValidateHTTPURL(req.TargetURL); err != nil {
		// err 已经是 AppError
		return err
	}

	// *_path：URL path
	if err := ValidateURLPath(req.MountPath, "mount_path"); err != nil {
		return err
	}
	if err := ValidateURLPath(req.UpstreamPath, "upstream_path"); err != nil {
		return err
	}

	// secret：nil 或空字符串不校验；否则必须 32 位小写 hex
	if req.Secret != nil {
		s := strings.TrimSpace(*req.Secret)
		if s != "" {
			hex32 := regexp.MustCompile(`^[a-f0-9]{32}$`)
			if !hex32.MatchString(s) {
				return tools.AppErrorNew(tools.InvalidArg, fmt.Sprintf("secret invalid: must be 32 lowercase hex chars, got %q", s), nil)
			}
		}
	}

	return nil
}

func AddApp(client gateflowv1.GateflowServiceClient, token string, req *AddAppReq, context context.Context) (string, error) {
	var err error

	context = AddAuth(context, token)
	err = AddAppVerify(req)

	if err != nil {
		return "", err
	}

	f_req := gateflowv1.AddAppRequest{
		AppName:      req.AppName,
		TargetUrl:    req.TargetURL,
		MountPath:    req.MountPath,
		UpstreamPath: req.UpstreamPath,
		Secret:       req.Secret,
	}

	f_res, err := client.AddApp(context, &f_req)
	if err != nil {
		return "", tools.AppError{
			Code:    tools.InvalidRequest,
			Message: "Could not AddApp",
			Err:     err,
		}
	}
	res, err := protojson.Marshal(f_res)

	if err != nil {
		return "", tools.AppError{
			Code:    tools.InvalidJson,
			Message: "Could not marshal json",
			Err:     err,
		}
	}

	return string(res), nil
}
