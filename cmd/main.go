package main

import (
	"log"

	"github.com/mammothbane/thulani-go"
)

func main() {
	conf, err := thulani.LoadConfig("config.yml")
	if err != nil {
		log.Fatal(err)
	}

	thulani.Run(conf)
}
