package thulani

import (
	"encoding/json"
	"os"

	"github.com/op/go-logging"
)

func handle(err error) {
	if err != nil {
		log.Fatal(err)
	}
}

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

func LoadConfig(filename string) (*Config, error) {
	file, err := os.Open("config.json")
	if err != nil {
		return nil, err
	}

	var conf Config
	err = json.NewDecoder(file).Decode(&conf)
	return &conf, err
}
