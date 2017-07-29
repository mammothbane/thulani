package thulani

import "github.com/mammothbane/thulani-go/downloader"

var cmdMap = map[string]func(*messageCtx){
	"help":   printHelp,
	"skip":   skip,
	"pause":  pause,
	"resume": resume,
	"sudoku": stop,
	"die":    stop,
	"list":   list,
	"queue":  list,
}

func printHelp(c *messageCtx) {
	c.sendMessage(help, c.Tts)
}

func skip(_ *messageCtx) {
	manager.PlayState <- downloader.Play
}

func resume(_ *messageCtx) {
	manager.PlayState <- downloader.Play
}

func pause(_ *messageCtx) {
	manager.PlayState <- downloader.Pause
}

func stop(_ *messageCtx) {
	manager.PlayState <- downloader.Clear
}

func list(_ *messageCtx) {
	log.Error("list not implemented")
}
