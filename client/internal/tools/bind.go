package tools

import (
	"fmt"
	"reflect"
	"strconv"
	"strings"
	"time"
)

// BindOpts: ALWAYS STRICT
func BindOpts[T any](opts map[string]string, dst *T) error {
	if dst == nil {
		return NewOptError("dst must be a non-nil pointer", nil)
	}

	rv := reflect.ValueOf(dst)
	if rv.Kind() != reflect.Pointer || rv.IsNil() {
		return NewOptError("dst must be a non-nil pointer", nil)
	}
	rv = rv.Elem()
	if rv.Kind() != reflect.Struct {
		return NewOptError(fmt.Sprintf("dst must point to a struct, got %s", rv.Kind()), nil)
	}

	// normalize incoming option keys to snake_case (underscore)
	nopts := make(map[string]string, len(opts))
	for k, v := range opts {
		nopts[normalizeKey(k)] = v
	}

	used := make(map[string]bool, len(nopts))
	rt := rv.Type()

	for i := 0; i < rt.NumField(); i++ {
		sf := rt.Field(i)

		// skip unexported
		if sf.PkgPath != "" {
			continue
		}
		// skip opt:"-"
		if sf.Tag.Get("opt") == "-" {
			continue
		}

		// key: opt tag or FieldName -> snake_case
		key := sf.Tag.Get("opt")
		if key == "" {
			key = toSnake(sf.Name)
		}
		key = normalizeKey(key)

		raw, ok := nopts[key]
		if !ok {
			if sf.Tag.Get("required") == "true" {
				return NewOptError(
					fmt.Sprintf("missing required option: --%s", strings.ReplaceAll(key, "_", "-")),
					nil,
				)
			}
			continue
		}
		used[key] = true

		fv := rv.Field(i)
		if !fv.CanSet() {
			continue
		}
		if err := setFromString(fv, raw); err != nil {
			return NewOptError(
				fmt.Sprintf("option --%s is invalid", strings.ReplaceAll(key, "_", "-")),
				err,
			)
		}
	}

	// ALWAYS STRICT: unknown options => error
	for k := range nopts {
		if !used[k] {
			return NewOptError(
				fmt.Sprintf("unknown option: --%s", strings.ReplaceAll(k, "_", "-")),
				nil,
			)
		}
	}

	return nil
}

func setFromString(v reflect.Value, s string) error {
	// pointer auto-alloc
	if v.Kind() == reflect.Pointer {
		if v.IsNil() {
			v.Set(reflect.New(v.Type().Elem()))
		}
		return setFromString(v.Elem(), s)
	}

	// time.Duration
	if v.Type() == reflect.TypeOf(time.Duration(0)) {
		d, err := time.ParseDuration(strings.TrimSpace(s))
		if err != nil {
			return NewOptError(fmt.Sprintf("invalid duration %q", s), err)
		}
		v.SetInt(int64(d))
		return nil
	}

	switch v.Kind() {
	case reflect.String:
		v.SetString(s)
		return nil

	case reflect.Bool:
		b, err := parseBool(s)
		if err != nil {
			return err
		}
		v.SetBool(b)
		return nil

	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		n, err := strconv.ParseInt(strings.TrimSpace(s), 10, v.Type().Bits())
		if err != nil {
			return NewOptError(fmt.Sprintf("invalid int %q", s), err)
		}
		v.SetInt(n)
		return nil

	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		n, err := strconv.ParseUint(strings.TrimSpace(s), 10, v.Type().Bits())
		if err != nil {
			return NewOptError(fmt.Sprintf("invalid uint %q", s), err)
		}
		v.SetUint(n)
		return nil

	case reflect.Float32, reflect.Float64:
		f, err := strconv.ParseFloat(strings.TrimSpace(s), v.Type().Bits())
		if err != nil {
			return NewOptError(fmt.Sprintf("invalid float %q", s), err)
		}
		v.SetFloat(f)
		return nil

	case reflect.Slice:
		parts := splitComma(s) // --tags=a,b,c
		slice := reflect.MakeSlice(v.Type(), 0, len(parts))
		for _, p := range parts {
			elem := reflect.New(v.Type().Elem()).Elem()
			if err := setFromString(elem, p); err != nil {
				return NewOptError(fmt.Sprintf("invalid slice element %q", p), err)
			}
			slice = reflect.Append(slice, elem)
		}
		v.Set(slice)
		return nil
	}

	return NewOptError(fmt.Sprintf("unsupported field type: %s", v.Type().String()), nil)
}

func parseBool(s string) (bool, error) {
	x := strings.ToLower(strings.TrimSpace(s))
	switch x {
	case "", "true", "1", "yes", "y", "on":
		return true, nil
	case "false", "0", "no", "n", "off":
		return false, nil
	default:
		return false, NewOptError(fmt.Sprintf("invalid bool %q", s), nil)
	}
}

func splitComma(s string) []string {
	s = strings.TrimSpace(s)
	if s == "" {
		return nil
	}
	raw := strings.Split(s, ",")
	out := make([]string, 0, len(raw))
	for _, p := range raw {
		p = strings.TrimSpace(p)
		if p != "" {
			out = append(out, p)
		}
	}
	return out
}

// FieldName -> snake_case (underscore), handles acronyms:
// AbPd -> ab_pd
// AppID -> app_id
// HTTPServer -> http_server
func toSnake(name string) string {
	var b strings.Builder
	r := []rune(name)

	isUpper := func(x rune) bool { return x >= 'A' && x <= 'Z' }
	isLower := func(x rune) bool { return x >= 'a' && x <= 'z' }
	isDigit := func(x rune) bool { return x >= '0' && x <= '9' }

	for i := 0; i < len(r); i++ {
		ch := r[i]

		// Insert '_' at boundaries:
		// 1) lower/digit -> Upper
		// 2) Upper -> Upper+lower (HTTPServer: before 'S')
		if i > 0 {
			prev := r[i-1]
			next := rune(0)
			if i+1 < len(r) {
				next = r[i+1]
			}

			if isUpper(ch) && (isLower(prev) || isDigit(prev)) {
				b.WriteByte('_')
			} else if isUpper(prev) && isUpper(ch) && next != 0 && isLower(next) {
				b.WriteByte('_')
			}
		}

		b.WriteRune(rune(strings.ToLower(string(ch))[0]))
	}

	return b.String()
}

// same rule as your ParseCommand:
//
//	target-url -> target_url
//	TARGET-URL -> target_url
func normalizeKey(k string) string {
	k = strings.TrimSpace(k)
	k = strings.ToLower(k)
	k = strings.ReplaceAll(k, "-", "_")
	return k
}

func BindOneOfStringOpts[T any](opts map[string]string, dst *T) error {
	if dst == nil {
		return NewOptError("dst must be a non-nil pointer", nil)
	}

	rv := reflect.ValueOf(dst)
	if rv.Kind() != reflect.Pointer || rv.IsNil() {
		return NewOptError("dst must be a non-nil pointer", nil)
	}
	rv = rv.Elem()
	if rv.Kind() != reflect.Struct {
		return NewOptError(fmt.Sprintf("dst must point to a struct, got %s", rv.Kind()), nil)
	}

	// normalize keys (same as ParseCommand)
	nopts := make(map[string]string, len(opts))
	for k, v := range opts {
		nopts[normalizeKey(k)] = v
	}

	rt := rv.Type()

	// 1) validate: all exported fields must be string
	for i := 0; i < rt.NumField(); i++ {
		sf := rt.Field(i)
		if sf.PkgPath != "" { // unexported: ignore
			continue
		}
		if sf.Tag.Get("opt") == "-" { // ignored field: still should be string? 你想严格的话可改成也校验
			continue
		}
		if sf.Type.Kind() != reflect.String {
			return NewOptError(
				fmt.Sprintf("all fields must be string, but %s is %s", sf.Name, sf.Type.String()),
				nil,
			)
		}
	}

	// 2) clear all exported fields to ""
	for i := 0; i < rt.NumField(); i++ {
		sf := rt.Field(i)
		if sf.PkgPath != "" || sf.Tag.Get("opt") == "-" {
			continue
		}
		fv := rv.Field(i)
		if fv.CanSet() {
			fv.SetString("")
		}
	}

	// 3) short-circuit bind: first matched key wins
	for i := 0; i < rt.NumField(); i++ {
		sf := rt.Field(i)
		if sf.PkgPath != "" || sf.Tag.Get("opt") == "-" {
			continue
		}

		key := sf.Tag.Get("opt")
		if key == "" {
			key = toSnake(sf.Name)
		}
		key = normalizeKey(key)

		if raw, ok := nopts[key]; ok {
			fv := rv.Field(i)
			if fv.CanSet() {
				fv.SetString(raw)
			}
			return nil
		}
	}

	// no match => keep all ""
	return nil
}
