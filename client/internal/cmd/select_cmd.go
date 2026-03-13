package cmd

import (
	"client/internal/tools"
	gateflowv1 "client/v1"
	"context"
	"fmt"
)

func SelectCmd(c *ParsedCmd, client gateflowv1.GateflowServiceClient, context context.Context) error {
	switch c.Mode {
	case "app":
		return selectAppCmd(c, client, context)
	case "route":
		return selectRouteCmd(c, client, context)
	case "login":
		return selectLoginCmd(c, client, context)
	default:
		return tools.NewCmdError()
	}
}

func selectLoginCmd(c *ParsedCmd, client gateflowv1.GateflowServiceClient, context context.Context) error {
	var err error
	var res string
	dto := LoginReq{}
	err = tools.BindOpts(c.Opts, &dto)
	if err != nil {
		return err
	}
	res, err = Login(client, &dto, context)
	if err != nil {
		return err
	}
	fmt.Println(res)
	return nil
}

func selectAppCmd(c *ParsedCmd, client gateflowv1.GateflowServiceClient, context context.Context) error {

	var err error
	var res string

	token, err := Get_token()

	if err != nil {
		return err
	}

	switch c.Sub {
	case "add":
		dto := AddAppReq{}
		err = tools.BindOpts(c.Opts, &dto)
		if err != nil {
			return err
		}
		res, err = AddApp(client, token, &dto, context)
		if err != nil {
			return err
		}
		fmt.Println(res)
	case "disable":
		dto := DisableAppReq{}
		err = tools.BindOneOfStringOpts(c.Opts, &dto)
		if err != nil {
			return err
		}
		res, err = DisableApp(client, token, &dto, context)
		if err != nil {
			return err
		}
		fmt.Println(res)
	case "approve":
		dto := AppoveAppReq{}
		err = tools.BindOneOfStringOpts(c.Opts, &dto)
		if err != nil {
			return err
		}
		res, err = AppoveApp(client, token, &dto, context)
		if err != nil {
			return err
		}
		fmt.Println(res)
	case "list":
		res, err = ListApps(client, token, context)
		if err != nil {
			return err
		}
		fmt.Println(res)
	case "show":
		dto := ShowAppReq{}
		err = tools.BindOneOfStringOpts(c.Opts, &dto)
		if err != nil {
			return err
		}
		res, err = ShowApp(client, token, &dto, context)
		if err != nil {
			return err
		}
		fmt.Println(res)
	default:
		return tools.NewCmdError()
	}
	return nil
}

func selectRouteCmd(c *ParsedCmd, client gateflowv1.GateflowServiceClient, context context.Context) error {
	var err error
	var res string
	token, err := Get_token()

	if err != nil {
		return err
	}
	switch c.Sub {
	case "update":
		dto := RouteUpdateReq{}
		err = tools.BindOpts(c.Opts, &dto)
		if err != nil {
			return err
		}
		res, err = RouteUpdate(client, token, &dto, context)
		if err != nil {
			return err
		}
		fmt.Println(res)
	default:
		return tools.NewCmdError()
	}
	return nil
}
