package cmd

import (
	"client/internal/tools"
	gateflowv1 "client/v1"
	"context"

	"google.golang.org/protobuf/encoding/protojson"
)

type DisableAppReq struct {
	AppID   string
	AppName string
}

func disableAppVerify(req DisableAppReq) error {
	if req.AppName == "" && req.AppID == "" {
		return tools.AppError{
			Code:    tools.InvalidArg,
			Message: "Argument is empty.",
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

func getDisappIdentify(req DisableAppReq) (*gateflowv1.DisableAppRequest, error) {
	err := disableAppVerify(req)
	if err != nil {
		return &gateflowv1.DisableAppRequest{}, err
	}
	if req.AppName != "" {
		return &gateflowv1.DisableAppRequest{
			AppIdentify:  req.AppName,
			IdentifyType: string(AppName),
		}, nil
	}

	return &gateflowv1.DisableAppRequest{
		AppIdentify:  req.AppID,
		IdentifyType: string(AppId),
	}, nil
}

func DisableApp(client gateflowv1.GateflowServiceClient, token string, req *DisableAppReq, context context.Context) (string, error) {
	context = AddAuth(context, token)
	f_req, err := getDisappIdentify(*req)
	if err != nil {
		return "", err
	}
	f_res, err := client.DisableApp(context, f_req)
	if err != nil {
		return "", tools.AppError{
			Code:    tools.InvalidRequest,
			Message: "Could not disable app.",
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
