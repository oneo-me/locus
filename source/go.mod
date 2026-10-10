module github.com/oneo-me/locus

go 1.27.1

require (
	github.com/egoist/mygo v0.3.8-0.20261010090026-0f79256101ac
	github.com/fsnotify/fsnotify v1.10.1
	github.com/modelcontextprotocol/go-sdk v1.8.0
	github.com/oneo-me/onevra/source v0.0.0
	github.com/yuin/goldmark v1.7.16
	go.yaml.in/yaml/v3 v3.0.4
)

require (
	github.com/alecthomas/chroma/v2 v2.27.0 // indirect
	github.com/dlclark/regexp2/v2 v2.2.1 // indirect
	github.com/ebitengine/purego v0.11.1 // indirect
	github.com/go-text/typesetting v0.3.5 // indirect
	github.com/google/jsonschema-go v0.4.3 // indirect
	github.com/nfnt/resize v0.0.0-20180221191011-83c6a9932646 // indirect
	github.com/segmentio/asm v1.1.3 // indirect
	github.com/segmentio/encoding v0.5.4 // indirect
	github.com/tc-hib/winres v0.3.1 // indirect
	github.com/yosida95/uritemplate/v3 v3.0.2 // indirect
	golang.org/x/image v0.46.0 // indirect
	golang.org/x/oauth2 v0.35.0 // indirect
	golang.org/x/sync v0.20.0 // indirect
	golang.org/x/sys v0.48.0 // indirect
	golang.org/x/time v0.15.0 // indirect
)

replace github.com/oneo-me/onevra/source => /Users/oneo/.oneo/neorio/worktrees/onevra-5adb910b/webview/source

replace github.com/egoist/mygo => /Users/oneo/.oneo/neorio/worktrees/onevra-5adb910b/webview/source/.mygo/mygo

tool github.com/oneo-me/onevra/source/tools/packer
