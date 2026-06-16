package wasi

import "errors"

// BufferStream wraps a byte slice as an InputStream.  Used by tests
// and as a fallback when streaming data is already materialized.
type BufferStream struct {
	data []byte
	off  int
}

// NewBufferStream returns an InputStream backed by data.
func NewBufferStream(data []byte) *BufferStream {
	return &BufferStream{data: data}
}

// Read implements InputStream.
func (s *BufferStream) Read(buf []byte) (int, bool, error) {
	if s.off >= len(s.data) {
		return 0, true, nil
	}
	n := copy(buf, s.data[s.off:])
	s.off += n
	eof := s.off >= len(s.data)
	return n, eof, nil
}

// CaptureStream is an OutputStream that accumulates writes in memory.
// Used by tests to inspect produced bodies.
type CaptureStream struct {
	buf    []byte
	closed bool
}

// NewCaptureStream returns an empty CaptureStream.
func NewCaptureStream() *CaptureStream { return &CaptureStream{} }

// Bytes returns the accumulated body bytes (copy-free).
func (s *CaptureStream) Bytes() []byte { return s.buf }

// WriteAll implements OutputStream.
func (s *CaptureStream) WriteAll(chunk []byte) error {
	if s.closed {
		return errors.New("stream closed")
	}
	s.buf = append(s.buf, chunk...)
	return nil
}

// Close marks the stream closed.  Subsequent WriteAll calls fail.
func (s *CaptureStream) Close() error { s.closed = true; return nil }
