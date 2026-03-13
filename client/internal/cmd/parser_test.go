package cmd

import "testing"

func TestParseCommand_ParsesModeSubOptsAndArgs(t *testing.T) {
	got := ParseCommand([]string{
		"app", "add",
		"--target-url=http://demo.internal",
		"--mount-path", "/demo",
		"extra-arg",
		"--dry-run",
	})

	if got.Mode != "app" {
		t.Fatalf("Mode = %q, want %q", got.Mode, "app")
	}
	if got.Sub != "add" {
		t.Fatalf("Sub = %q, want %q", got.Sub, "add")
	}
	if got.Opts["target_url"] != "http://demo.internal" {
		t.Fatalf("target_url = %q, want %q", got.Opts["target_url"], "http://demo.internal")
	}
	if got.Opts["mount_path"] != "/demo" {
		t.Fatalf("mount_path = %q, want %q", got.Opts["mount_path"], "/demo")
	}
	if got.Opts["dry_run"] != "true" {
		t.Fatalf("dry_run = %q, want %q", got.Opts["dry_run"], "true")
	}
	if len(got.Args) != 1 || got.Args[0] != "extra-arg" {
		t.Fatalf("Args = %#v, want [extra-arg]", got.Args)
	}
}

func TestParseCommand_EmptyInput(t *testing.T) {
	got := ParseCommand(nil)
	if got.Mode != "" || got.Sub != "" {
		t.Fatalf("unexpected parsed mode/sub: %#v", got)
	}
	if len(got.Opts) != 0 {
		t.Fatalf("Opts should be empty, got %#v", got.Opts)
	}
	if len(got.Args) != 0 {
		t.Fatalf("Args should be empty, got %#v", got.Args)
	}
}
