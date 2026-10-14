package main

import (
	"os"
	"regexp"
	"slices"
	"strconv"
	"strings"
	"testing"

	"go.yaml.in/yaml/v3"
)

func TestTranslateFollowsPreferenceAndLocale(t *testing.T) {
	cases := []struct {
		preference string
		locale     string
		want       string
	}{
		{"en", "zh-Hant-TW", "Settings"},
		{"english", "zh-Hant-TW", "Settings"},
		{"zh-Hans", "ja-JP", "设置"},
		{"chinese", "ja-JP", "设置"},
		{"zh-Hant", "en-US", "設定"},
		{"traditional-chinese", "en-US", "設定"},
		{"ja", "zh-CN", "設定"},
		{"japanese", "zh-CN", "設定"},
		{"ko", "en-US", "설정"},
		{"korean", "en-US", "설정"},
		{"system", "zh-Hans-CN", "设置"},
		{"system", "zh-CN", "设置"},
		{"system", "zh-SG", "设置"},
		{"system", "zh", "设置"},
		{"system", "zh-Hans-TW", "设置"},
		{"system", "zh-Hant-TW", "設定"},
		{"system", "zh-Hant-HK", "設定"},
		{"system", "zh-TW", "設定"},
		{"system", "zh-HK", "設定"},
		{"system", "zh-MO", "設定"},
		{"system", "zh_TW.UTF-8", "設定"},
		{"system", "zh-Hant-CN", "設定"},
		{"system", "ja-JP", "設定"},
		{"system", "ja_JP.UTF-8", "設定"},
		{"system", "ko-KR", "설정"},
		{"system", "ko_KR.UTF-8", "설정"},
		{"system", "en-US", "Settings"},
		{"system", "yue-Hant-HK", "Settings"},
		{"system", "C", "Settings"},
		{"system", "POSIX", "Settings"},
		{"system", "", "Settings"},
		{"", "ko", "설정"},
		{"fr", "zh-TW", "設定"},
	}
	for _, tc := range cases {
		if got := translate(tc.preference, tc.locale, "Settings"); got != tc.want {
			t.Errorf("translate(%q, %q) = %q, want %q", tc.preference, tc.locale, got, tc.want)
		}
	}
	if got := translate("ja", "ja-JP", "Not a product string"); got != "Not a product string" {
		t.Fatalf("missing entries should stay English, got %q", got)
	}
	preserved := map[string]string{
		"Settings":           "设置",
		"system":             "跟随系统",
		"none":               "无（仅 Shell）",
		"Apply connection":   "应用连接设置",
		"Modified files":     "已修改的文件",
		"visual-studio-code": "VS Code",
		"Agents":             "Agent",
	}
	for key, want := range preserved {
		if got := translate("zh-Hans", "", key); got != want {
			t.Errorf("simplified %q = %q, want %q", key, got, want)
		}
	}
}

func TestUpdaterLanguage(t *testing.T) {
	if updaterLanguage("zh-Hans") != "zh-Hans" || updaterLanguage("chinese") != "zh-Hans" || updaterLanguage("ja") != "ja" {
		t.Fatal("explicit language")
	}
	if updaterLanguage("system") != "" || updaterLanguage("") != "" || updaterLanguage("fr") != "" {
		t.Fatal("system language")
	}
}

func TestLanguageCatalogs(t *testing.T) {
	type message struct {
		Other string `yaml:"other"`
	}
	catalogs := map[string]map[string]message{}
	for _, code := range catalogCodes {
		data, err := localeFiles.ReadFile("locales/" + code + ".yaml")
		if err != nil {
			t.Fatal(err)
		}
		var entries map[string]message
		if err := yaml.Unmarshal(data, &entries); err != nil {
			t.Fatal(err)
		}
		catalogs[code] = entries
	}
	formats := regexp.MustCompile(`%(?:\[[0-9]+\])?[-+ #0]*[0-9]*(?:\.[0-9]+)?[a-zA-Z%]`)
	english := catalogs["en"]
	for _, code := range catalogCodes {
		if code == "en" {
			continue
		}
		if len(catalogs[code]) != len(english) {
			t.Errorf("%s has %d messages, English has %d", code, len(catalogs[code]), len(english))
		}
		for id, en := range english {
			got, ok := catalogs[code][id]
			if !ok || got.Other == "" || en.Other == "" {
				t.Errorf("%s missing %s", code, id)
				continue
			}
			a, b := formats.FindAllString(en.Other, -1), formats.FindAllString(got.Other, -1)
			slices.Sort(a)
			slices.Sort(b)
			if !slices.Equal(a, b) {
				t.Errorf("%s format mismatch %s", code, id)
			}
		}
	}
	simplifiedOnly := "设简库记笔载复导显标传败现读终默认随浅观题语辑录侧应装编写启态无仅视刷进链须请没见选开关闭写国门"
	for id, message := range catalogs["zh-Hant"] {
		if strings.Contains(message.Other, "简体中文") {
			continue
		}
		for _, r := range message.Other {
			if strings.ContainsRune(simplifiedOnly, r) {
				t.Errorf("traditional %s = %q contains simplified %q", id, message.Other, string(r))
			}
		}
	}
}

func TestInterfaceStringsAreTranslated(t *testing.T) {
	literal := regexp.MustCompile(`a\.text\("((?:\\.|[^"\\])*)"\)`)
	entries, err := os.ReadDir(".")
	if err != nil {
		t.Fatal(err)
	}
	found := 0
	for _, entry := range entries {
		name := entry.Name()
		if entry.IsDir() || !strings.HasSuffix(name, ".go") || strings.HasSuffix(name, "_test.go") {
			continue
		}
		body, err := os.ReadFile(name)
		if err != nil {
			t.Fatal(err)
		}
		for _, match := range literal.FindAllStringSubmatch(string(body), -1) {
			key, err := strconv.Unquote(`"` + match[1] + `"`)
			if err != nil {
				t.Fatal(err)
			}
			found++
			if _, ok := sourceMessageIDs[key]; !ok {
				t.Errorf("%s: uncatalogued %q", name, key)
			}
		}
	}
	if found == 0 {
		t.Fatal("found no interface strings")
	}
	for _, key := range []string{
		"Agent", "Editor", "Status",
		"Install authoring skill", "Install authoring skill globally",
		"system", "light", "dark", "default", "none", "visual-studio-code",
		"Include at least one image in the gallery.",
		"Include only local images with alt text in the gallery.",
	} {
		if _, ok := sourceMessageIDs[key]; !ok {
			t.Errorf("uncatalogued %q", key)
		}
	}
}
