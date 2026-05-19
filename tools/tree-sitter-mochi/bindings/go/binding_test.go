package tree_sitter_mochi_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_mochi "github.com/darrigu/mochi/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_mochi.Language())
	if language == nil {
		t.Errorf("Error loading Mochi grammar")
	}
}
