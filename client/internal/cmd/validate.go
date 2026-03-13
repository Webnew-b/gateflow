package cmd

import (
	"client/internal/tools"
	"fmt"
	"net/url"
	"path"
	"regexp"
	"strings"

	"github.com/google/uuid"
)

var slugRe = regexp.MustCompile(`^[a-z0-9-]+$`)

func IsLowerAlphaNumDash(s string) bool {
	return s != "" && slugRe.MatchString(s)
}

var uuid36Lower = regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`)

func IsStrictUUIDv7(s string) bool {
	if !uuid36Lower.MatchString(s) {
		return false
	}
	u, err := uuid.Parse(s)
	if err != nil {
		return false
	}
	return u.Variant() == uuid.RFC4122 && u.Version() == 7
}

func IsUUID(s string) bool {
	if s == "" {
		return false
	}
	_, err := uuid.Parse(s)
	return err == nil
}

func ValidateHTTPURL(raw string) error {
	if strings.ContainsAny(raw, " \t\r\n") {
		return tools.AppErrorNew(tools.InvalidArg, "target_url must not contain whitespace", nil)
	}
	u, err := url.Parse(raw)
	if err != nil {
		return tools.AppErrorNew(tools.InvalidArg, "target_url parse failed", err)
	}
	if u.Scheme != "http" && u.Scheme != "https" {
		return tools.AppErrorNew(tools.InvalidArg, fmt.Sprintf("target_url scheme must be http/https, got %q", u.Scheme), nil)
	}
	if u.Host == "" {
		return tools.AppErrorNew(tools.InvalidArg, "target_url host is required", nil)
	}
	return nil
}

func ValidateURLPath(p string, field string) error {
	if strings.ContainsAny(p, " \t\r\n") {
		return tools.AppErrorNew(tools.InvalidArg, fmt.Sprintf("%s must not contain whitespace", field), nil)
	}
	if !strings.HasPrefix(p, "/") {
		return tools.AppErrorNew(tools.InvalidArg, fmt.Sprintf("%s must start with '/'", field), nil)
	}
	if strings.ContainsAny(p, "?#") {
		return tools.AppErrorNew(tools.InvalidArg, fmt.Sprintf("%s must not contain '?' or '#'", field), nil)
	}

	u, err := url.Parse("http://example.com" + p)
	if err != nil {
		return tools.AppErrorNew(tools.InvalidArg, fmt.Sprintf("%s parse failed", field), err)
	}
	if u.Path != p {
		return tools.AppErrorNew(tools.InvalidArg, fmt.Sprintf("%s normalized mismatch: parsed=%q", field, u.Path), nil)
	}

	clean := path.Clean(p)
	if clean != p {
		return tools.AppErrorNew(tools.InvalidArg, fmt.Sprintf("%s must be clean path (no '.'/'..' or redundant segments), got %q -> %q", field, p, clean), nil)
	}
	return nil
}
