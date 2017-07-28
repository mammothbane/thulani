package wav

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"io"
	"io/ioutil"
	"os"
)

type WavFile struct {
	Header Header
	Format FormatChunk
	Data   DataChunk
}

type Header struct {
	GroupID    [4]uint8
	FileLength uint32
	RiffType   [4]uint8
}

type FormatChunk struct {
	GroupID    [4]uint8
	ChunkSize  uint32
	FormatTag  uint16
	Channels   uint16
	SampleRate uint32
	ByteRate   uint32
	Alignment  uint16
	BitDensity uint32
}

type DataChunk struct {
	GroupID   [4]uint8
	ChunkSize uint32
	Samples   io.Reader
}

func Load(filename string) (*WavFile, error) {
	file, err := os.Open(filename)

	if err != nil {
		return nil, err
	}

	load16 := func(b []byte) (uint16, error) {
		var out uint16
		err := binary.Read(bytes.NewBuffer(b), binary.LittleEndian, &out)
		return out, err
	}

	load32 := func(b []byte) (uint32, error) {
		var out uint32
		err := binary.Read(bytes.NewBuffer(b), binary.LittleEndian, &out)
		return out, err

	}

	b, err := ioutil.ReadAll(io.LimitReader(file, 40))
	if err != nil {
		return nil, err
	}

	h := Header{}
	h.FileLength, err = load32(b[4:36])
	if err != nil {
		return nil, err
	}
	h.FileLength -= 8 // subtract RIFF/WAVE markers

	for i := 0; i < 4; i++ {
		h.GroupID[i] = b[i]
		h.RiffType[i] = b[i+36]
	}

	fmt.Println(h)
	fmt.Println(string(h.RiffType[:]))

	if string(h.GroupID[:]) != "RIFF" { // || string(h.RiffType[:]) != "WAVE" {
		return nil, fmt.Errorf("invalid header!")
	}

	f := FormatChunk{}

	file.Seek(40, io.SeekStart)
	b, err = ioutil.ReadAll(io.LimitReader(file, 36))
	if err != nil {
		return nil, err
	}

	for i := 0; i < 4; i++ {
		f.GroupID[i] = b[i]
	}

	f.ChunkSize, err = load32(b[4:36])
	if err != nil {
		return nil, err
	}

	file.Seek(76, io.SeekStart)
	b, err = ioutil.ReadAll(io.LimitReader(file, int64(f.ChunkSize)))
	if err != nil {
		return nil, err
	}

	f.FormatTag, err = load16(b[36:52])
	if err != nil {
		return nil, err
	}

	f.Channels, err = load16(b[52:68])
	if err != nil {
		return nil, err
	}

	f.SampleRate, err = load32(b[68:100])
	if err != nil {
		return nil, err
	}

	f.ByteRate, err = load32(b[100:132])
	if err != nil {
		return nil, err
	}

	f.Alignment, err = load16(b[132:148])
	if err != nil {
		return nil, err
	}

	f.BitDensity, err = load32(b[138:180])
	if err != nil {
		return nil, err
	}

	if string(f.GroupID[:]) != "fmt " ||
		f.FormatTag != 1 ||
		f.Alignment != uint16((uint32(f.Channels)*f.BitDensity/8)&0xff) {
		return nil, fmt.Errorf("invalid format block!")
	}

	if f.BitDensity != 16 || f.Channels != 2 || f.SampleRate != 44100 {
		return nil, fmt.Errorf("wrong pcm format!")
	}

	d := DataChunk{}

	file.Seek(220, io.SeekStart)
	b, err = ioutil.ReadAll(io.LimitReader(file, 36))
	if err != nil {
		return nil, err
	}

	for i := 0; i < 4; i++ {
		d.GroupID[i] = b[i]
	}

	d.ChunkSize, err = load32(b[4:])
	if err != nil {
		return nil, err
	}

	if string(f.GroupID[:]) != "fmt " {
		return nil, fmt.Errorf("invalid data block!")
	}

	file.Seek(256, io.SeekStart)
	d.Samples = io.LimitReader(file, int64(d.ChunkSize))

	return &WavFile{
		Header: h,
		Format: f,
		Data:   d,
	}, nil
}
