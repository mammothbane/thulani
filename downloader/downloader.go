package downloader

import (
	"net/url"
	"os/exec"
	"strconv"
	"time"

	"io/ioutil"
	"os"

	"fmt"

	"github.com/mammothbane/thulani-go/wav"
	"github.com/op/go-logging"
)

var log = logging.MustGetLogger("downloader")

func getUrl(inUrl string) (string, error) {
	dl := exec.Command("youtube-dl", "-f", "bestaudio", "-x", "--get-url", inUrl)

	b, err := dl.CombinedOutput()
	if err != nil {
		log.Errorf("youtube-dl failed: %v", string(b))
		return "", err
	}

	tgt, err := url.Parse(string(b))
	out := tgt.Scheme + "://" + tgt.Host + tgt.Path + "?" + tgt.Query().Encode()

	return out, nil
}

func Download(inUrl string, startTime time.Duration, duration time.Duration) (<-chan []byte, error) {
	targetUrl, err := getUrl(inUrl)
	if err != nil {
		return nil, err
	}

	startSecond := int(startTime.Seconds())
	dur := int(duration.Seconds())

	args := []string{
		"-ss", strconv.Itoa(startSecond),
		"-i", targetUrl,
		"-c:a", "pcm_s16le",
		"-f", "wav",
		"-ar", "48000",
		"-ac", "2",
		"-vn", "-y",
	}

	if dur > 0 {
		args = append(args, "-t", strconv.Itoa(dur))
	}

	file, err := ioutil.TempFile("", "thulani_")
	if err != nil {
		return nil, err
	}

	clearTemp := func() {
		if err := os.Remove(file.Name()); err != nil {
			log.Errorf("unable to remove temp file: %q", err)
		}
	}

	args = append(args, file.Name())
	//args = append(args, "out.wav")

	fmt.Println(args)

	dl := exec.Command(`ffmpeg`, args...)
	b, err := dl.CombinedOutput()
	if err != nil {
		clearTemp()
		log.Errorf("ffmpeg failed: \n%v", string(b))
		return nil, err
	}

	ch, done, err := wav.Load(file.Name())
	if err != nil {
		clearTemp()
		return nil, err
	}

	go func() {
		<-done
		clearTemp()
	}()

	return ch, err
}
