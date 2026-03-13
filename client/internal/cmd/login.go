package cmd

import (
	"client/internal/tools"
	gateflowv1 "client/v1"
	"context"

	"google.golang.org/protobuf/encoding/protojson"
)

type LoginReq struct {
	Username string
	Password string
}

func LoginVerify(req LoginReq) error {
	if req.Password == "" || req.Username == "" {
		return tools.AppError{
			Code:    tools.InvalidArg,
			Message: "Login data is null.",
			Err:     nil,
		}
	}
	return nil
}

func Login(client gateflowv1.GateflowServiceClient, req *LoginReq, context context.Context) (string, error) {
	f_req := gateflowv1.LoginRequest{
		Password: req.Password,
		Username: req.Username,
	}

	f_res, err := client.Login(context, &f_req)
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
			Code:    tools.InvalidRequest,
			Message: "Could not login account",
			Err:     err,
		}
	}

	return string(res), nil
}
