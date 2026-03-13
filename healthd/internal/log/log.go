package log

import (
	stdlog "log"
	"os"
)

var logger = stdlog.New(os.Stderr, "", stdlog.LstdFlags|stdlog.Lshortfile)

func Print(v ...any) {
	logger.Print(v...)
}

func Printf(format string, v ...any) {
	logger.Printf(format, v...)
}

func Println(v ...any) {
	logger.Println(v...)
}
