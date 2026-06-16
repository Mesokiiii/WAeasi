package handler

import (
	"encoding/json"
	"strconv"

	"waeasi.dev/sdk/wasi"
)

// Response is the Fetch-style response a handler returns.
type Response struct {
	Status     int
	StatusText string
	Headers    *Headers
	body       []byte // owned bytes; nil for empty
	stream     wasi.InputStream // optional streaming body source
}

// NewResponse builds a Response with the given status and byte body.
// Content-Length is auto-injected.
func NewResponse(status int, body []byte) *Response {
	if status < 100 || status > 599 {
		panic("handler.NewResponse: status out of range")
	}
	return &Response{
		Status:  status,
		Headers: NewHeaders(4),
		body:    body,
	}
}

// JSON marshals v with encoding/json, sets the Content-Type, returns
// a 200 Response (override .Status afterwards if needed).
func JSON(v any) *Response {
	b, err := json.Marshal(v)
	if err != nil {
		return ErrorResponse(500, "json marshal: "+err.Error())
	}
	r := NewResponse(200, b)
	r.Headers.Set("content-type", "application/json; charset=utf-8")
	return r
}

// Text returns a text/plain response with the given body.
func Text(status int, msg string) *Response {
	r := NewResponse(status, []byte(msg))
	r.Headers.Set("content-type", "text/plain; charset=utf-8")
	return r
}

// Redirect emits a 3xx with a Location header.  Allowed: 301/302/303/307/308.
func Redirect(location string, status int) *Response {
	switch status {
	case 301, 302, 303, 307, 308:
	default:
		panic("handler.Redirect: invalid status")
	}
	r := NewResponse(status, nil)
	r.Headers.Set("location", location)
	return r
}

// ErrorResponse is a convenience for 4xx/5xx with a plain-text body.
func ErrorResponse(status int, msg string) *Response {
	r := NewResponse(status, []byte(msg))
	r.Headers.Set("content-type", "text/plain; charset=utf-8")
	return r
}

// Stream replaces this Response's body with the contents of rx.
// Disables Content-Length unless caller has set it explicitly.
func (r *Response) Stream(rx wasi.InputStream) *Response {
	r.body = nil
	r.stream = rx
	return r
}

func (r *Response) writeToOutparam(out wasi.ResponseOutparam) error {
	if !r.Headers.Has("content-length") && !r.Headers.Has("transfer-encoding") {
		if r.stream == nil {
			r.Headers.Set("content-length", strconv.Itoa(len(r.body)))
		}
	}

	pairs := make([][2]any, 0, r.Headers.Len())
	r.Headers.Range(func(name, value string) bool {
		pairs = append(pairs, [2]any{name, []byte(value)})
		return true
	})

	tx, err := out.StartResponse(r.Status, pairs)
	if err != nil {
		return err
	}
	defer tx.Close()
	defer out.Finish()

	if r.stream != nil {
		buf := make([]byte, 8*1024)
		for {
			n, eof, err := r.stream.Read(buf)
			if err != nil {
				return err
			}
			if n > 0 {
				if err := tx.WriteAll(buf[:n]); err != nil {
					return err
				}
			}
			if eof {
				return nil
			}
		}
	}
	if len(r.body) == 0 {
		return nil
	}
	return tx.WriteAll(r.body)
}
