package tools

import "fmt"

type ErrCode string

const (
	Unauthorized   ErrCode = "Unauthorized"
	InvalidJson    ErrCode = "Invalid json"
	InvalidArg     ErrCode = "Invalid argument"
	InvalidRequest ErrCode = "Invalid request"
	InvalidCommand ErrCode = "Invalid Commadn"
	InvalidOpt     ErrCode = "Invalid Option"
	InvalidEnv     ErrCode = "Invalid environment"
	Other          ErrCode = "Other"
)

type AppError struct {
	Code    ErrCode
	Message string
	Err     error
}

func (e AppError) Error() string {
	if e.Err != nil {
		return fmt.Sprintf("code=%s message=%s:%v", e.Code, e.Message, e.Err)
	}
	return fmt.Sprintf("code=%s message=%s", e.Code, e.Message)
}

func AppErrorNew(code ErrCode, msg string, err error) AppError {
	return AppError{
		Code:    code,
		Message: msg,
		Err:     err,
	}
}

func NewCmdError() AppError {
	return AppError{
		Code:    InvalidCommand,
		Message: "Unknown command",
		Err:     nil,
	}
}

func NewOptError(msg string, err error) AppError {
	return AppError{
		Code:    InvalidOpt,
		Message: msg,
		Err:     err,
	}
}
