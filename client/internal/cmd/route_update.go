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

type RouteUpdateReq struct {
	AppName      string
	MountPath    string
	UpstreamPath string
}

func RouteUpdateVerify(req RouteUpdateReq) error {

	req.AppName = strings.TrimSpace(req.AppName)
	req.MountPath = strings.TrimSpace(req.MountPath)
	req.UpstreamPath = strings.TrimSpace(req.UpstreamPath)

	// 判空：属于请求不完整
	if req.AppName == "" {
		return tools.AppErrorNew(tools.InvalidRequest, "app_name is required", nil)
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

	// *_path：URL path
	if err := ValidateURLPath(req.MountPath, "mount_path"); err != nil {
		return err
	}
	if err := ValidateURLPath(req.UpstreamPath, "upstream_path"); err != nil {
		return err
	}

	return nil
}

func RouteUpdate(client gateflowv1.GateflowServiceClient, token string, req *RouteUpdateReq, context context.Context) (string, error) {
	context = AddAuth(context, token)
	f_req := gateflowv1.RouteUpdateRequest{
		AppName:      req.AppName,
		MountPath:    req.MountPath,
		UpstreamPath: req.UpstreamPath,
	}

	f_res, err := client.RouteUpdate(context, &f_req)
	if err != nil {
		return "", tools.AppError{
			Code:    tools.InvalidRequest,
			Message: "Route Update fail",
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
