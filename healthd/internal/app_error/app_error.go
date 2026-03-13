package app_error

import (
	"errors"
	"fmt"
)

// AppError：最简结构，只保留 Code + Message + Err（错误链全靠 Err 传递）
type AppError struct {
	Code    Code
	Message string
	Err     error
}

func (e *AppError) Error() string {
	if e == nil {
		return "<nil>"
	}

	// 错误链字符串自然会由 Err 的 Error() 递归展开
	// e.g. code=RPC_FAILED msg=pull targets: rpc error...
	if e.Err != nil && e.Message != "" {
		return fmt.Sprintf("code=%s msg=%s: %v", e.Code, e.Message, e.Err)
	}
	if e.Err != nil {
		return fmt.Sprintf("code=%s: %v", e.Code, e.Err)
	}
	if e.Message != "" {
		return fmt.Sprintf("code=%s msg=%s", e.Code, e.Message)
	}
	return fmt.Sprintf("code=%s", e.Code)
}

// Unwrap 让 errors.Is / errors.As / errors.Unwrap 能沿 Err 走完整链
func (e *AppError) Unwrap() error { return e.Err }

// Is：允许用 errors.Is(err, &AppError{Code: CodeXXX}) 匹配错误码
// 不比较 Message，避免不稳定。
func (e *AppError) Is(target error) bool {
	t, ok := target.(*AppError)
	if !ok {
		return false
	}
	return t.Code != "" && e.Code == t.Code
}

// New：只有 code + message（无底层 err）
func New(code Code, message string) *AppError {
	return &AppError{Code: code, Message: message}
}

// Wrap：code + message + err（err 形成错误链）
func Wrap(code Code, err error, message string) *AppError {
	if err == nil {
		return &AppError{Code: code, Message: message}
	}
	return &AppError{Code: code, Message: message, Err: err}
}

// WithCode：给任意 err “标注”一个 code（不丢失原链）
// message 可空；常用于把第三方错误统一归类。
func WithCode(code Code, err error, message string) error {
	if err == nil {
		return nil
	}
	return Wrap(code, err, message)
}

// CodeOf：从错误链中提取最外层（最近）的 AppError.Code；找不到则 INTERNAL
func CodeOf(err error) Code {
	if err == nil {
		return CodeOK
	}
	var ae *AppError
	if errors.As(err, &ae) && ae.Code != "" {
		return ae.Code
	}
	return CodeInternal
}

func IsCode(err error, code Code) bool {
	if err == nil {
		return false
	}
	return errors.Is(err, &AppError{Code: code})
}
