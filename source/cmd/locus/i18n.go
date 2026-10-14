package main

import (
	"embed"
	"strings"
	"sync"

	"github.com/egoist/mygo"
	"github.com/nicksnyder/go-i18n/v2/i18n"
	"go.yaml.in/yaml/v3"
	"golang.org/x/text/language"
)

// languageOptions are the values Settings stores. Language names are endonyms
// drawn by languageLabel, not catalog entries. Older files may still say
// english or chinese; explicitCatalog accepts those names.
var languageOptions = []string{"system", "en", "zh-Hans", "zh-Hant", "ja", "ko"}

var catalogCodes = []string{"en", "zh-Hans", "zh-Hant", "ja", "ko"}

//go:embed locales/*.yaml
var localeFiles embed.FS

var englishMessages, sourceMessageIDs = loadEnglishMessages()
var messages = loadMessages()

func loadEnglishMessages() (map[string]string, map[string]string) {
	data, err := localeFiles.ReadFile("locales/en.yaml")
	if err != nil {
		panic(err)
	}
	var catalog map[string]struct {
		Other string `yaml:"other"`
	}
	if err := yaml.Unmarshal(data, &catalog); err != nil {
		panic(err)
	}
	english, sources := map[string]string{}, map[string]string{}
	for id, message := range catalog {
		if message.Other == "" {
			panic("empty English message: " + id)
		}
		if _, exists := sources[message.Other]; exists {
			panic("duplicate English message: " + message.Other)
		}
		english[id] = message.Other
		sources[message.Other] = id
	}
	return english, sources
}

func loadMessages() *i18n.Bundle {
	bundle := i18n.NewBundle(language.English)
	bundle.RegisterUnmarshalFunc("yaml", yaml.Unmarshal)
	for _, code := range catalogCodes {
		data, err := localeFiles.ReadFile("locales/" + code + ".yaml")
		if err != nil {
			panic(err)
		}
		if _, err := bundle.ParseMessageFileBytes(data, code+".yaml"); err != nil {
			panic(err)
		}
	}
	return bundle
}

var (
	localizersMu sync.Mutex
	localizers   = map[string]*i18n.Localizer{}
)

func localizer(code string) *i18n.Localizer {
	localizersMu.Lock()
	defer localizersMu.Unlock()
	if existing, ok := localizers[code]; ok {
		return existing
	}
	created := i18n.NewLocalizer(messages, code, "en")
	localizers[code] = created
	return created
}

// explicitCatalog maps a stored preference to a catalog. Legacy names stay
// readable. system and unrecognized values are not explicit.
func explicitCatalog(preference string) (string, bool) {
	switch preference {
	case "en", "english":
		return "en", true
	case "zh-Hans", "chinese":
		return "zh-Hans", true
	case "zh-Hant", "traditional-chinese":
		return "zh-Hant", true
	case "ja", "japanese":
		return "ja", true
	case "ko", "korean":
		return "ko", true
	default:
		return "", false
	}
}

// catalogForLocale maps a MyGo locale to a catalog. An explicit Hant script
// stays Traditional even when a region follows, and Hans stays Simplified.
// TW, HK, and MO are Traditional when the tag has no script. Other Chinese
// tags, including a bare zh, use Simplified Chinese.
func catalogForLocale(locale string) string {
	locale = strings.TrimSpace(locale)
	locale = strings.ReplaceAll(locale, "_", "-")
	if cut := strings.IndexAny(locale, ".@"); cut >= 0 {
		locale = locale[:cut]
	}
	if locale == "" || strings.EqualFold(locale, "C") || strings.EqualFold(locale, "POSIX") {
		return "en"
	}
	tag, err := language.Parse(locale)
	if err != nil {
		return "en"
	}
	base, _ := tag.Base()
	switch base.String() {
	case "ja":
		return "ja"
	case "ko":
		return "ko"
	case "zh":
		script, _ := tag.Script()
		switch script.String() {
		case "Hant":
			return "zh-Hant"
		case "Hans":
			return "zh-Hans"
		}
		region, _ := tag.Region()
		switch region.String() {
		case "TW", "HK", "MO":
			return "zh-Hant"
		default:
			return "zh-Hans"
		}
	default:
		return "en"
	}
}

// translate resolves an explicit preference, or the application locale when
// the preference is system or unrecognized. Missing entries stay in English.
func translate(preference, locale, message string) string {
	code := "en"
	if explicit, ok := explicitCatalog(preference); ok {
		code = explicit
	} else {
		code = catalogForLocale(locale)
	}
	return lookup(code, message)
}

func lookup(code, message string) string {
	id, ok := sourceMessageIDs[message]
	if !ok {
		return message
	}
	fallback := englishMessages[id]
	if code == "" || code == "en" {
		return fallback
	}
	text, err := localizer(code).Localize(&i18n.LocalizeConfig{
		MessageID:      id,
		DefaultMessage: &i18n.Message{ID: id, Other: fallback},
	})
	if err != nil && text == "" {
		return fallback
	}
	return text
}

func (a *application) text(message string) string {
	locale := ""
	if _, ok := explicitCatalog(a.cfg.Language); !ok {
		locale = mygo.App.Locale()
	}
	return translate(a.cfg.Language, locale, message)
}

func (a *application) languageLabel(code string) string {
	switch code {
	case "en", "english":
		return "English"
	case "zh-Hans", "chinese":
		return "简体中文"
	case "zh-Hant", "traditional-chinese":
		return "繁體中文"
	case "ja", "japanese":
		return "日本語"
	case "ko", "korean":
		return "한국어"
	case "", "system":
		return a.text("system")
	default:
		return a.text(code)
	}
}
