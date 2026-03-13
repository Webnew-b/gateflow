package config

import (
	"os"
	"path/filepath"
	"time"

	"gopkg.in/yaml.v3"

	"healthd/internal/app_error"
)

const DefaultConfigFilename = "healthd.yaml"

const ExampleYAML = `# healthd.yaml

# 从 Gateway 拉健康目标列表的 gRPC 地址
gateway_admin_rpc_endpoint: "http://127.0.0.1:9000"
# Gateway 登录后得到的 session token（用于 NodeList 鉴权）
gateway_session_token: "replace-with-login-session-token"

# 任务周期（核心两个周期）
# - health_check_every：每隔多久对 targets 做一次 HTTP 探测 + UDP 上报
# - targets_sync_every：每隔多久从 Gateway 重新拉一次 targets 列表（gRPC Pull）
health_check_every: "10s"
targets_sync_every: "5m"

# UDP 上报目标（Gateway 的 UDP listener）
gateway_udp_addr: "127.0.0.1:8123"

# HTTP 探测参数
http:
  timeout: "3s"                 # 单次探测超时
  user_agent: "gateflow-healthd/0.1"

# 并发控制（避免同时探测太多打爆自己或上游）
concurrency:
  max_in_flight: 64

# 日志
log:
  level: "info"
`

type Config struct {
	GatewayAdminRPCEndpoint string `yaml:"gateway_admin_rpc_endpoint"`
	GatewaySessionToken     string `yaml:"gateway_session_token"`
	HealthCheckEvery        string `yaml:"health_check_every"`
	TargetsSyncEvery        string `yaml:"targets_sync_every"`
	GatewayUDPAddr          string `yaml:"gateway_udp_addr"`

	HTTP struct {
		Timeout   string `yaml:"timeout"`
		UserAgent string `yaml:"user_agent"`
	} `yaml:"http"`

	Concurrency struct {
		MaxInFlight int `yaml:"max_in_flight"`
	} `yaml:"concurrency"`

	Log struct {
		Level string `yaml:"level"`
	} `yaml:"log"`
}

type RuntimeConfig struct {
	GatewayAdminRPCEndpoint string
	GatewaySessionToken     string
	CheckInterval           time.Duration
	SyncInterval            time.Duration
	GatewayUDPAddr          string

	HTTPTimeout   time.Duration
	HTTPUserAgent string

	MaxInFlight int
	LogLevel    string
}

func FindProjectRoot() (string, error) {
	start, err := os.Getwd()
	if err != nil {
		return "", app_error.Wrap(app_error.CodeIOFailed, err, "get working directory failed")
	}

	dir := start
	for {
		if _, err := os.Stat(filepath.Join(dir, "go.mod")); err == nil {
			return dir, nil
		}

		parent := filepath.Dir(dir)
		if parent == dir {
			// 到达文件系统根目录也没找到 go.mod
			return "", app_error.New(app_error.CodeInvalidConfig, "project root not found (missing go.mod in parent dirs)")
		}
		dir = parent
	}
}

func EnsureExampleConfigExists() (cfgPath string, created bool, err error) {
	root, err := FindProjectRoot()
	if err != nil {
		return "", false, err
	}

	cfgPath = filepath.Join(root, DefaultConfigFilename)

	_, statErr := os.Stat(cfgPath)
	if statErr == nil {
		return cfgPath, false, nil
	}
	if !os.IsNotExist(statErr) {
		return "", false, app_error.Wrap(app_error.CodeIOFailed, statErr, "stat config file failed")
	}

	if writeErr := os.WriteFile(cfgPath, []byte(ExampleYAML), 0644); writeErr != nil {
		return "", false, app_error.Wrap(app_error.CodeIOFailed, writeErr, "write example config failed")
	}

	return cfgPath, true, nil
}

func LoadConfigFromProjectRoot() (RuntimeConfig, error) {
	root, err := FindProjectRoot()
	if err != nil {
		return RuntimeConfig{}, err
	}

	cfgPath := filepath.Join(root, DefaultConfigFilename)

	b, readErr := os.ReadFile(cfgPath)
	if readErr != nil {
		return RuntimeConfig{}, app_error.Wrap(app_error.CodeIOFailed, readErr, "read config file failed")
	}

	var raw Config
	if unmarshalErr := yaml.Unmarshal(b, &raw); unmarshalErr != nil {
		return RuntimeConfig{}, app_error.Wrap(app_error.CodeDecodeFailed, unmarshalErr, "parse yaml config failed")
	}

	return normalize(raw)
}

func normalize(raw Config) (RuntimeConfig, error) {
	// 必填项校验
	if raw.GatewayAdminRPCEndpoint == "" {
		return RuntimeConfig{}, app_error.New(app_error.CodeInvalidConfig, "gateway_admin_rpc_endpoint is empty")
	}
	if raw.GatewaySessionToken == "" {
		return RuntimeConfig{}, app_error.New(app_error.CodeInvalidConfig, "gateway_session_token is empty")
	}
	if raw.GatewayUDPAddr == "" {
		return RuntimeConfig{}, app_error.New(app_error.CodeInvalidConfig, "gateway_udp_addr is empty")
	}
	if raw.HealthCheckEvery == "" {
		return RuntimeConfig{}, app_error.New(app_error.CodeInvalidConfig, "health_check_every is empty")
	}
	if raw.TargetsSyncEvery == "" {
		return RuntimeConfig{}, app_error.New(app_error.CodeInvalidConfig, "targets_sync_every is empty")
	}
	if raw.HTTP.Timeout == "" {
		return RuntimeConfig{}, app_error.New(app_error.CodeInvalidConfig, "http.timeout is empty")
	}
	if raw.Concurrency.MaxInFlight <= 0 {
		return RuntimeConfig{}, app_error.New(app_error.CodeInvalidConfig, "concurrency.max_in_flight must be > 0")
	}

	// duration 解析
	checkInterval, err := time.ParseDuration(raw.HealthCheckEvery)
	if err != nil {
		return RuntimeConfig{}, app_error.Wrap(app_error.CodeInvalidConfig, err, "invalid health_check_every (duration)")
	}
	syncInterval, err := time.ParseDuration(raw.TargetsSyncEvery)
	if err != nil {
		return RuntimeConfig{}, app_error.Wrap(app_error.CodeInvalidConfig, err, "invalid targets_sync_every (duration)")
	}
	httpTimeout, err := time.ParseDuration(raw.HTTP.Timeout)
	if err != nil {
		return RuntimeConfig{}, app_error.Wrap(app_error.CodeInvalidConfig, err, "invalid http.timeout (duration)")
	}

	out := RuntimeConfig{
		GatewayAdminRPCEndpoint: raw.GatewayAdminRPCEndpoint,
		GatewaySessionToken:     raw.GatewaySessionToken,
		CheckInterval:           checkInterval,
		SyncInterval:            syncInterval,
		GatewayUDPAddr:          raw.GatewayUDPAddr,

		HTTPTimeout:   httpTimeout,
		HTTPUserAgent: raw.HTTP.UserAgent,

		MaxInFlight: raw.Concurrency.MaxInFlight,
		LogLevel:    raw.Log.Level,
	}
	return out, nil
}

func CanRun(N, maxGo int) (mode string, ok bool, need int) {
	if N <= 0 {
		return "none", true, 0
	}
	if maxGo >= N {
		return "all", true, N
	}
	groups := (N + 7) / 8
	need = 4 * groups
	if maxGo >= need {
		return "per_batch_4", true, need
	}
	return "reject", false, need
}
