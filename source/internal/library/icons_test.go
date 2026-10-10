package library

import "testing"

func TestScanIconMetadata(t *testing.T) {
	root := t.TempDir()
	for name, value := range map[string]string{"emoji": "'👩🏽‍💻'", "image": "assets/icon.png", "bool": "true", "object": "{path: assets/icon.png}"} {
		put(t, root, name+"/index.md", "---\nicon: "+value+"\n---\nBody")
	}
	lib := openTest(t, root)
	notes, err := lib.Scan()
	if err != nil {
		t.Fatal(err)
	}
	for name, want := range map[string]string{"emoji": "👩🏽‍💻", "image": "assets/icon.png", "bool": "", "object": ""} {
		if got := Find(notes, name).Icon; got != want {
			t.Errorf("%s icon = %q, want %q", name, got, want)
		}
	}
}
