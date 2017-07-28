package thulani

import (
	"math/rand"
	"regexp"
	"strings"
	"time"
)

func init() {
	rand.Seed(time.Now().UnixNano())
}

var extraMemes = []func(*messageCtx) MemeStatus{
	respondToFuckYou,
	respondToMeme,
	respondToRaaaaaaaaaaaay,
}

var hateMatch = []string{
	"suck",
	"fuck",
	"trash",
	"garbage",
	"stupid",
	"shit",
	"dick",
	"bitch",
	"hate",
}

var responses = []string{
	"WELL FUCK YOU TOO YOU PIECE OF SHIT",
	"**i'll fucking burst ye**",
	"memememexexxxxxxxxxxxwerp",
	"thulando madondo",
	"you are a memerman",
}

type MemeStatus int

const (
	Continue MemeStatus = iota
	Interrupt
)

func respondToFuckYou(ctx *messageCtx) (result MemeStatus) {
	result = Continue
	content := strings.ToLower(ctx.Message.Content)

	if !strings.Contains(content, config.Trigger) {
		return
	}

	for _, v := range hateMatch {
		if strings.Contains(content, strings.ToLower(v)) {
			response := responses[rand.Intn(len(responses))]

			ctx.sendMessage(response, true)
			return
		}
	}

	return
}

func respondToMeme(ctx *messageCtx) MemeStatus {
	if !(ctx.Matched && ctx.Command == "meme") {
		return Continue
	}

	ctx.sendMessage("i am not yet capable of memeing.", false)
	return Interrupt
}

var ray = regexp.MustCompile("ra+y")

// TODO: play the sound clip
func respondToRaaaaaaaaaaaay(ctx *messageCtx) MemeStatus {
	if ctx.Matched && ray.MatchString(ctx.Command) {
		ctx.sendMessage(ctx.Command, true)
		return Interrupt
	}

	return Continue
}
