package thulani

var cmdMap = map[string]func(*messageCtx){
	"help":   printHelp,
	"skip":   commandNotImplemented,
	"pause":  commandNotImplemented,
	"resume": commandNotImplemented,
	"sudoku": commandNotImplemented,
	"die":    commandNotImplemented,
	"list":   commandNotImplemented,
	"queue":  commandNotImplemented,
}

func printHelp(c *messageCtx) {
	c.sendMessage(help, c.Tts)
}

func commandNotImplemented(c *messageCtx) {
	log.Errorf("%q not implemented", c.Command)
	c.sendMessage("not implemented", c.Tts)
}

func skip(c *messageCtx) {
	log.Error("skip not implemented")
}

func resume(c *messageCtx) {
	log.Error("skip not implemented")
}

func pause(c *messageCtx) {
	log.Error("skip not implemented")
}

func stop(c *messageCtx) {
	log.Error("skip not implemented")
}

func list(c *messageCtx) {
	log.Error("skip not implemented")
}
