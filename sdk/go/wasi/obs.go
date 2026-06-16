package wasi

import (
	"fmt"
	"os"
)

// LogLevel mirrors waeasi:obs/log.level.
type LogLevel uint8

const (
	LogTrace LogLevel = iota
	LogDebug
	LogInfo
	LogWarn
	LogError
)

func (l LogLevel) String() string {
	switch l {
	case LogTrace: return "trace"
	case LogDebug: return "debug"
	case LogInfo:  return "info"
	case LogWarn:  return "warn"
	case LogError: return "error"
	}
	return "?"
}

// LogEmit is the package-private hook the build pipeline replaces with
// a direct host call.  In dev mode we route to stderr.
var LogEmit = func(level LogLevel, target, msg string) {
	fmt.Fprintf(os.Stderr, "[%s] %s: %s\n", level, target, msg)
}

// LogEnabled returns true if the host wants to receive logs at this
// level.  Dev fallback is permissive.
var LogEnabled = func(level LogLevel, target string) bool { return true }

// CounterAdd / GaugeSet / HistogramObserve are no-ops in dev mode and
// patched by the build pipeline.
var (
	CounterAdd        = func(handle uint64, by int64)            {}
	GaugeSet          = func(handle uint64, value int64)         {}
	HistogramObserve  = func(handle uint64, value float64)        {}
	RegisterCounter   = func(name string) uint64                  { return 0 }
	RegisterGauge     = func(name string) uint64                  { return 0 }
	RegisterHistogram = func(name string, buckets []float64) uint64 { return 0 }
)
