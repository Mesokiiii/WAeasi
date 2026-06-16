package handler

import (
	"encoding/json"
	"errors"
	"io"

	"waeasi.dev/sdk/wasi"
)

// Body is a one-shot streaming body adapter.  Methods that consume the
// body (Bytes, Text, JSON) may be called at most once each, and only
// one of them may be called overall.
type Body struct {
	consumed bool
	buf      []byte
	stream   wasi.InputStream
}

func bodyFromWasi(rx wasi.InputStream) *Body {
	return &Body{stream: rx}
}

// NewBody wraps an in-memory byte slice as a Body (used by tests).
func NewBody(data []byte) *Body { return &Body{buf: data} }

func (b *Body) guard() error {
	if b == nil {
		return errors.New("nil body")
	}
	if b.consumed {
		return errors.New("body already consumed")
	}
	b.consumed = true
	return nil
}

// Bytes drains the body to a single allocation.
func (b *Body) Bytes() ([]byte, error) {
	if err := b.guard(); err != nil {
		return nil, err
	}
	if b.buf != nil {
		return b.buf, nil
	}
	return drain(b.stream)
}

// Text reads the body as UTF-8.
func (b *Body) Text() (string, error) {
	bs, err := b.Bytes()
	if err != nil {
		return "", err
	}
	return string(bs), nil
}

// JSON decodes the body as JSON into v.
func (b *Body) JSON(v any) error {
	bs, err := b.Bytes()
	if err != nil {
		return err
	}
	return json.Unmarshal(bs, v)
}

// CopyTo streams the body to w and returns the number of bytes copied.
func (b *Body) CopyTo(w io.Writer) (int64, error) {
	if err := b.guard(); err != nil {
		return 0, err
	}
	if b.buf != nil {
		n, err := w.Write(b.buf)
		return int64(n), err
	}
	var total int64
	buf := make([]byte, 8*1024)
	for {
		n, eof, err := b.stream.Read(buf)
		if err != nil {
			return total, err
		}
		if n > 0 {
			m, werr := w.Write(buf[:n])
			total += int64(m)
			if werr != nil {
				return total, werr
			}
		}
		if eof {
			return total, nil
		}
	}
}

func drain(rx wasi.InputStream) ([]byte, error) {
	if rx == nil {
		return nil, nil
	}
	var out []byte
	buf := make([]byte, 8*1024)
	for {
		n, eof, err := rx.Read(buf)
		if err != nil {
			return out, err
		}
		if n > 0 {
			out = append(out, buf[:n]...)
		}
		if eof {
			return out, nil
		}
	}
}
