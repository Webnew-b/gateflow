package cmd

import "strings"

type ParsedCmd struct {
	Mode string            // e.g. "app"
	Sub  string            // e.g. "add"
	Opts map[string]string // e.g. target_url -> "http://..."
	Args []string          // positional args
}

// ParseCommand parses: client <mode> [sub] [--k v|--k=v|--flag] [args...]
func ParseCommand(argv []string) ParsedCmd {
	out := ParsedCmd{
		Opts: map[string]string{},
	}

	// argv 约定包含 os.Args[1:]（不含程序名）
	if len(argv) == 0 {
		return out
	}

	out.Mode = argv[0]
	i := 1

	// 第二段如果不是 flag，就当 subcommand
	if i < len(argv) && !strings.HasPrefix(argv[i], "-") {
		out.Sub = argv[i]
		i++
	}

	for i < len(argv) {
		tok := argv[i]

		// 不是 flag -> 位置参数
		if !strings.HasPrefix(tok, "-") {
			out.Args = append(out.Args, tok)
			i++
			continue
		}

		// 只处理 --xxx / -x 形式（你也可以选择只允许 --）
		key := strings.TrimLeft(tok, "-")
		if key == "" {
			i++
			continue
		}

		// 支持 --k=v
		if strings.Contains(key, "=") {
			parts := strings.SplitN(key, "=", 2)
			k := normalizeKey(parts[0])
			v := parts[1]
			out.Opts[k] = v
			i++
			continue
		}

		k := normalizeKey(key)

		// 支持 --flag (bool true)
		// 如果下一个 token 不存在或是另一个 flag，就当 true
		if i+1 >= len(argv) || strings.HasPrefix(argv[i+1], "-") {
			out.Opts[k] = "true"
			i++
			continue
		}

		// 支持 --k v
		out.Opts[k] = argv[i+1]
		i += 2
	}

	return out
}
