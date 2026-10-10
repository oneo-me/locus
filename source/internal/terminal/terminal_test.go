package terminal

import (
	"os/exec"
	"strings"
	"testing"
)

func TestQuoting(t *testing.T) {
	if _, err := exec.LookPath("sh"); err != nil {
		t.Skip(err)
	}
	for _, value := range []string{"中文 path", "'single'", "$(touch should-not-exist)", "line\nsecond", "back\\slash"} {
		data, err := exec.Command("sh", "-c", "printf %s "+Quote(value)).Output()
		if err != nil || string(data) != value {
			t.Fatalf("quote %q -> %q %v", value, data, err)
		}
	}
}
func TestLaunchCommandValidation(t *testing.T) {
	root := t.TempDir()
	if _, err := Command(root, "bash", "unknown;exit"); err == nil {
		t.Fatal("Agent injection accepted")
	}
	if _, err := Command(root, "bash;exit", "codex"); err == nil {
		t.Fatal("shell injection accepted")
	}
	if _, err := exec.LookPath("bash"); err != nil {
		t.Skip(err)
	}
	command, err := Command(root, "bash", "codex")
	if err != nil || !strings.Contains(command, "codex") || !strings.Contains(command, " -lic ") {
		t.Fatalf("command %q %v", command, err)
	}
	// Deliberately do not execute the terminal/Agent launch command.
}
