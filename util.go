package thulani

import (
	"encoding/json"
	"os"

	"github.com/op/go-logging"
)

const help = `wew lad. you should know these commands already.

Usage: ` + "`!thulani [command]`" + `

commands:
**help**:\t\t\t\tprint this help message
**[url]**:\t\t\t   a url with media that thulani can play. queued up to play after everything that's already waiting.
**list, queue**:\tlist items in the queue, as well as the currently-playing item.
**pause**:\t\t\tpause sound.
**resume**:\t\t resume sound.
**die**:\t\t\t\t empty the queue and stop playing.
**skip**:\t\t\t   skip the current item.
`

var log = logging.MustGetLogger("thulani")

type Config struct {
	Trigger      string `json:"trigger"`
	QueueSize    uint   `json:"queue_size"`
	Admin        uint   `json:"admin"`
	OpRole       string `json:"op_role"`
	Server       string `json:"server"`
	VoiceChannel string `json:"voice_channel"`
	Token        string `json:"token"`
}

func handle(err error) {
	if err != nil {
		log.Fatal(err)
	}
}

func LoadConfig(filename string) (*Config, error) {
	file, err := os.Open("config.json")
	if err != nil {
		return nil, err
	}

	var conf Config
	err = json.NewDecoder(file).Decode(&conf)
	return &conf, err
}
