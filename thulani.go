package main

import (
	"encoding/json"
	"log"
	"os"

	"github.com/bwmarrin/discordgo"
)

type config struct {
	Trigger      string `json:"trigger"`
	QueueSize    uint   `json:"queue_size"`
	Admin        string `json:"admin"`
	OpRole       string `json:"op_role"`
	Server       string `json:"server"`
	VoiceChannel string `json:"voice_channel"`
	Token        string `json:"token"`
}

func (c *config) UnmarshalYAML(unmarshal func(interface{}) error) error {

	return nil
}

func main() {
	file, err := os.Open("config.json")
	handle(err)

	var conf config
	handle(json.NewDecoder(file).Decode(&conf))

	dg, err := discordgo.New()
	handle(err)

	app := &discordgo.Application{}
	app.Name = "Thulani"

}

func handle(err error) {
	if err != nil {
		log.Fatal(err)
	}
}
