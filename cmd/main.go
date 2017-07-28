package main

import (
	"log"

	"github.com/mammothbane/thulani-go"
)

func main() {
	conf, err := thulani.LoadConfig("config.json")
	if err != nil {
		log.Fatal(err)
	}

	thulani.Run(conf)
}
