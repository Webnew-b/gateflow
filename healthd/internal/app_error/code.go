package app_error

type Code string

const (
	// 通用
	CodeOK       Code = "OK"
	CodeInternal Code = "INTERNAL"

	// 配置/参数
	CodeInvalidConfig Code = "INVALID_CONFIG"
	CodeInvalidEnv    Code = "INVALID_ENV"
	CodeInvalidArg    Code = "INVALID_ARG"

	// 序列化/解析
	CodeEncodeFailed Code = "ENCODE_FAILED"
	CodeDecodeFailed Code = "DECODE_FAILED"

	// RPC（healthd -> gateway admin rpc）
	CodeRPCFailed  Code = "RPC_FAILED"
	CodeRPCTimeout Code = "RPC_TIMEOUT"

	// HTTP（healthd -> app health endpoint）
	CodeHTTPFailed  Code = "HTTP_FAILED"
	CodeHTTPTimeout Code = "HTTP_TIMEOUT"

	// UDP（healthd -> gateway udp listener）
	CodeUDPFailed Code = "UDP_FAILED"

	// I/O（读配置文件等）
	CodeIOFailed Code = "IO_FAILED"
)
