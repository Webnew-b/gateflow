package cmd

import (
	"client/internal/tools"
	gateflowv1 "client/v1"
	"context"

	"google.golang.org/protobuf/encoding/protojson"
)

type AppoveAppReq struct {
	AppID   string
	AppName string
}

func appoveVerify(req AppoveAppReq) error {
	if req.AppName == "" && req.AppID == "" {
		return tools.AppError{
			Code:    tools.InvalidArg,
			Message: "either app_name or app_id is required.",
			Err:     nil,
		}
	}
	if req.AppID != "" && !IsUUID(req.AppID) {
		return tools.AppError{
			Code:    tools.InvalidArg,
			Message: "App id is invalid.",
			Err:     nil,
		}
	}
	if req.AppName != "" && !IsLowerAlphaNumDash(req.AppName) {
		return tools.AppError{
			Code:    tools.InvalidArg,
			Message: "App name is invalid.",
			Err:     nil,
		}
	}
	return nil
}

func getIdentify(req AppoveAppReq) (*gateflowv1.ApproveAppRequest, error) {
	err := appoveVerify(req)
	if err != nil {
		return &gateflowv1.ApproveAppRequest{}, err
	}
	if req.AppName != "" {
		return &gateflowv1.ApproveAppRequest{
			AppIdentify:  req.AppName,
			IdentifyType: string(AppName),
		}, nil
	}

	return &gateflowv1.ApproveAppRequest{
		AppIdentify:  req.AppID,
		IdentifyType: string(AppId),
	}, nil
}

func AppoveApp(client gateflowv1.GateflowServiceClient, token string, req *AppoveAppReq, context context.Context) (string, error) {
	context = AddAuth(context, token)
	f_req, err := getIdentify(*req)
	if err != nil {
		return "", err
	}
	f_res, err := client.ApproveApp(context, f_req)
	if err != nil {
		return "", tools.AppError{
			Code:    tools.InvalidRequest,
			Message: "login fail",
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
