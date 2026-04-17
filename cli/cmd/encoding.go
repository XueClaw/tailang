package cmd

import (
	"fmt"
	"unicode/utf8"
)

func decodeUTF8Source(content []byte) (string, error) {
	if len(content) >= 2 {
		if (content[0] == 0xFF && content[1] == 0xFE) || (content[0] == 0xFE && content[1] == 0xFF) {
			return "", fmt.Errorf("input file must use UTF-8, UTF-16 is forbidden")
		}
	}
	if !isUTF8(content) {
		return "", fmt.Errorf("input file must use UTF-8, GBK/ANSI/UTF-16 and other encodings are forbidden")
	}
	return string(content), nil
}

func isUTF8(content []byte) bool {
	for len(content) > 0 {
		r, size := utf8.DecodeRune(content)
		if r == utf8.RuneError && size == 1 {
			return false
		}
		content = content[size:]
	}
	return true
}
