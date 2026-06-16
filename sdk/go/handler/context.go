package handler

import (
	"encoding/json"
	"fmt"

	"waeasi.dev/sdk/wasi"
)

// Context carries per-invocation observability primitives.
//
// User handlers can ignore Context entirely and the harness will
// route logs to the default target.  The richer shape — with metrics
// and timing — is opt-in:
//
//	func handler(req *handler.Request) *handler.Response {
//	    ctx := handler.CtxFromRequest(req)
//	    ctx.Log.Info("got request", handler.F("path", req.Path()))
//	    ...
//	}
type Context struct {
	Target string
	Log    *Logger
}

// Logger is a structured logger that fans out to waeasi:obs/log.
type Logger struct {
	target string
}

// Field is a logging key/value attachment.
type Field struct {
	Key   string
	Value any
}

// F builds a Field with minimal allocation.
func F(key string, value any) Field { return Field{Key: key, Value: value} }

// CtxFromRequest returns a Context tagged with the global handler target.
func CtxFromRequest(_ *Request) *Context {
	return &Context{Target: target, Log: &Logger{target: target}}
}

func (l *Logger) emit(level wasi.LogLevel, msg string, fields []Field) {
	if !wasi.LogEnabled(level, l.target) {
		return
	}
	if len(fields) == 0 {
		wasi.LogEmit(level, l.target, msg)
		return
	}
	m := make(map[string]any, len(fields))
	for _, f := range fields {
		m[f.Key] = f.Value
	}
	b, err := json.Marshal(m)
	if err != nil {
		wasi.LogEmit(level, l.target, msg+" {marshal-error}")
		return
	}
	wasi.LogEmit(level, l.target, msg+" "+string(b))
}

// Trace / Debug / Info / Warn / Error level helpers.
func (l *Logger) Trace(msg string, fs ...Field) { l.emit(wasi.LogTrace, msg, fs) }
func (l *Logger) Debug(msg string, fs ...Field) { l.emit(wasi.LogDebug, msg, fs) }
func (l *Logger) Info(msg string, fs ...Field)  { l.emit(wasi.LogInfo, msg, fs) }
func (l *Logger) Warn(msg string, fs ...Field)  { l.emit(wasi.LogWarn, msg, fs) }
func (l *Logger) Error(msg string, fs ...Field) { l.emit(wasi.LogError, msg, fs) }

// Errorf is a fmt.Errorf-shaped logger that returns the formatted error
// for chained `return ctx.Log.Errorf(...)` patterns.
func (l *Logger) Errorf(format string, args ...any) error {
	err := fmt.Errorf(format, args...)
	l.Error(err.Error())
	return err
}
