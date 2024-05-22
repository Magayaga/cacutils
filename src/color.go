package main

type ANSIColors struct{}

func (ANSIColors) green(text string) string {
    return "\033[32m" + text + "\033[0m"
}

func (ANSIColors) blue(text string) string {
    return "\033[34m" + text + "\033[0m"
}

func (ANSIColors) yellow(text string) string {
    return "\033[33m" + text + "\033[0m"
}

func (ANSIColors) magenta(text string) string {
    return "\033[35m" + text + "\033[0m"
}

func (ANSIColors) cyan(text string) string {
    return "\033[36m" + text + "\033[0m"
}

