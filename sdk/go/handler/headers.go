package handler

import (
	"strings"
)

// Headers is an insertion-stable, case-insensitive multi-value map.
// API mirrors net/http.Header but is intentionally smaller and built
// on a slice to avoid Go map iteration overhead in the hot path.
type Headers struct {
	pairs []headerPair
}

type headerPair struct {
	name  string // already lower-cased
	value string
}

// NewHeaders creates an empty header set with optional capacity hint.
func NewHeaders(hint int) *Headers {
	return &Headers{pairs: make([]headerPair, 0, hint)}
}

// Append adds a value for name; duplicates are preserved in order.
func (h *Headers) Append(name, value string) {
	if err := validateName(name); err != nil {
		panic(err)
	}
	if err := validateValue(value); err != nil {
		panic(err)
	}
	h.pairs = append(h.pairs, headerPair{strings.ToLower(name), value})
}

// Set replaces every existing value for name with a single value.
func (h *Headers) Set(name, value string) {
	lc := strings.ToLower(name)
	if err := validateName(name); err != nil {
		panic(err)
	}
	if err := validateValue(value); err != nil {
		panic(err)
	}
	out := h.pairs[:0]
	for _, p := range h.pairs {
		if p.name != lc {
			out = append(out, p)
		}
	}
	h.pairs = append(out, headerPair{lc, value})
}

// Get returns the first matching value, or "" if none.
func (h *Headers) Get(name string) string {
	lc := strings.ToLower(name)
	for _, p := range h.pairs {
		if p.name == lc {
			return p.value
		}
	}
	return ""
}

// GetAll returns every matching value (slice is a copy).
func (h *Headers) GetAll(name string) []string {
	lc := strings.ToLower(name)
	var out []string
	for _, p := range h.pairs {
		if p.name == lc {
			out = append(out, p.value)
		}
	}
	return out
}

// Has reports whether at least one value exists for name.
func (h *Headers) Has(name string) bool {
	lc := strings.ToLower(name)
	for _, p := range h.pairs {
		if p.name == lc {
			return true
		}
	}
	return false
}

// Delete removes every value for name.
func (h *Headers) Delete(name string) {
	lc := strings.ToLower(name)
	out := h.pairs[:0]
	for _, p := range h.pairs {
		if p.name != lc {
			out = append(out, p)
		}
	}
	h.pairs = out
}

// Range iterates over every (name, value) pair in insertion order.
func (h *Headers) Range(fn func(name, value string) bool) {
	for _, p := range h.pairs {
		if !fn(p.name, p.value) {
			return
		}
	}
}

// Len returns the total number of values (sum across all names).
func (h *Headers) Len() int { return len(h.pairs) }

func validateName(name string) error {
	if name == "" {
		return errInvalidHeader
	}
	for i := 0; i < len(name); i++ {
		c := name[i]
		if c < 0x21 || c > 0x7E {
			return errInvalidHeader
		}
		switch c {
		case '(', ')', ',', '/', ':', ';', '<', '=', '>', '?', '@',
			'[', '\\', ']', '{', '}', '"':
			return errInvalidHeader
		}
	}
	return nil
}

func validateValue(v string) error {
	for i := 0; i < len(v); i++ {
		c := v[i]
		if c == '\r' || c == '\n' || c == 0 {
			return errInvalidHeader
		}
	}
	return nil
}

var errInvalidHeader = headerError("invalid header name or value")

type headerError string

func (e headerError) Error() string { return string(e) }
