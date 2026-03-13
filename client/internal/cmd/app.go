package cmd

import (
	"client/internal/tools"
	"context"
	"os"
	"strings"

	"google.golang.org/grpc/metadata"
)

type AppIdentify string

const (
	AppName AppIdentify = "app_name"
	AppId   AppIdentify = "app_id"
)

func Get_token() (string, error) {
	v, ok := os.LookupEnv("APP_TOKEN")
	if !ok {
		return "", tools.AppError{
			Code:    tools.InvalidEnv,
			Message: "Login Token is Invalid",
			Err:     nil,
		}
	}
	return v, nil
}

func GetHost() (string, error) {
	v, ok := os.LookupEnv("APP_HOST")
	if !ok {
		return "", tools.AppError{
			Code:    tools.InvalidEnv,
			Message: "Login Token is Invalid",
			Err:     nil,
		}
	}
	return v, nil
}

func AddAuth(ctx context.Context, token string) context.Context {
	md := metadata.Pairs("authorization", "Bearer "+token)
	ctx = metadata.NewOutgoingContext(ctx, md)
	return ctx
}

// normalizeKey converts:
//
//	target-url -> target_url
//	TARGET-URL -> target_url
func normalizeKey(k string) string {
	k = strings.TrimSpace(k)
	k = strings.ToLower(k)
	k = strings.ReplaceAll(k, "-", "_")
	return k
}
