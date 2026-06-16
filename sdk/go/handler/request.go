package handler

import (
	"net/url"
	"strings"

	"waeasi.dev/sdk/wasi"
)

// Request is the Fetch-style request handed to user handlers.
//
// The body is exposed as a *Body that supports one-shot Bytes() / Text() /
// JSON() consumption.  Re-reading the body is forbidden (matches WHATWG
// Fetch and Cloudflare Workers semantics).
type Request struct {
	Method      string
	URL         string
	Headers     *Headers
	Traceparent string
	body        *Body
}

// Body returns the request body for streaming or buffered consumption.
func (r *Request) Body() *Body { return r.body }

// Path returns the URL path component (no query, no host).
func (r *Request) Path() string {
	u, err := url.Parse(r.URL)
	if err != nil {
		return "/"
	}
	if u.Path == "" {
		return "/"
	}
	return u.Path
}

// Host returns the host component of r.URL.
func (r *Request) Host() string {
	u, err := url.Parse(r.URL)
	if err != nil {
		return ""
	}
	return u.Host
}

// Query returns parsed URL query parameters.
func (r *Request) Query() url.Values {
	u, err := url.Parse(r.URL)
	if err != nil {
		return url.Values{}
	}
	return u.Query()
}

// HeaderEqual reports whether the named header equals want (case-insens).
func (r *Request) HeaderEqual(name, want string) bool {
	return strings.EqualFold(r.Headers.Get(name), want)
}

func requestFromWasi(req wasi.IncomingRequest) *Request {
	scheme := req.Scheme()
	authority := req.Authority()
	path := req.PathWithQuery()

	urlStr := scheme + "://" + authority + path
	headers := NewHeaders(req.HeadersLen())
	req.RangeHeaders(func(name string, value []byte) bool {
		headers.Append(name, string(value))
		return true
	})

	tp := headers.Get("traceparent")
	body := bodyFromWasi(req.ConsumeBody())

	return &Request{
		Method:      req.Method(),
		URL:         urlStr,
		Headers:     headers,
		Traceparent: tp,
		body:        body,
	}
}
