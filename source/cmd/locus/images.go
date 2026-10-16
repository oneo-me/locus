package main

import (
	"bytes"
	"encoding/binary"
	"errors"
	"image"
	"image/color"
	_ "image/gif"
	_ "image/jpeg"
	_ "image/png"
	"math"
	"net/url"
	"path"
	"strings"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"golang.org/x/image/draw"
)

// Local images are decoded outside frame builders and kept under a pixel
// budget: one note retains at most imageTotalPixels, and each picture is
// scaled down to fit rather than dropped. imageDecodeLimit guards against
// decoding a pathologically large source before scaling it.
const (
	imageDecodeLimit = 64_000_000
	imageMaxPixels   = 4_000_000
	imageTotalPixels = 48_000_000
	imageMinPixels   = 250_000
)

func resourcePath(note, target string) (string, error) {
	if strings.HasPrefix(target, "locus://") {
		return library.DecodeResource(strings.TrimPrefix(target, "locus://"))
	}
	u, err := url.Parse(target)
	if err != nil {
		return "", err
	}
	if u.Scheme != "" || u.Host != "" {
		return "", nil
	}
	// Decode the whole destination, not EscapedPath: MyGo passes the
	// destination as written, and a file name may contain # or ?. Percent
	// escapes still decode exactly once.
	decoded, err := url.PathUnescape(target)
	if err != nil {
		return "", err
	}
	p := path.Join(note, decoded)
	return library.DecodeResource(strings.TrimPrefix(library.Address(p), "locus://"))
}
func (a *application) loadImages() {
	key := a.model.Root + "\x00" + a.model.Selected + "\x00" + a.model.Document.Body
	if key == a.imageKey {
		return
	}
	a.imageKey = key
	root, note, body := a.model.Root, a.model.Selected, a.model.Document.Body
	a.images = nil
	a.galleryPreviewOpen = false
	ready := make(chan struct{})
	a.imageMu.Lock()
	a.imageTicket++
	ticket := a.imageTicket
	a.imageReady = ready
	a.imageLoad = nil
	a.imageMu.Unlock()
	if root == "" || note == "" || body == "" {
		close(ready)
		return
	}
	go func() {
		images := map[string]*ui.Bitmap{}
		pixels := int64(0)
		lib, err := library.Open(root)
		if err != nil {
			close(ready)
			return
		}
		defer lib.Close()
		_, targets := markdownResources(body)
		seen := map[string]bool{}
		unique := make([]string, 0, len(targets))
		for _, target := range targets {
			if seen[target] {
				continue
			}
			seen[target] = true
			if len(unique) >= 64 {
				break
			}
			unique = append(unique, target)
		}
		for i, target := range unique {
			p, err := resourcePath(note, target)
			if err != nil || p == "" {
				continue
			}
			data, err := lib.Resource(p)
			if err != nil {
				continue
			}
			size, _, err := image.DecodeConfig(bytes.NewReader(data))
			if err != nil || size.Width <= 0 || size.Height <= 0 {
				continue
			}
			source := int64(size.Width) * int64(size.Height)
			if source > imageDecodeLimit {
				continue
			}
			// Spread the note's budget over the pictures still to load so every
			// one gets room instead of the later ones running out of it.
			limit := (int64(imageTotalPixels) - pixels) / int64(len(unique)-i)
			limit = min(limit, int64(imageMaxPixels))
			limit = max(limit, int64(imageMinPixels))
			bitmap, retained, err := decodeGalleryImage(data, source, limit)
			if err != nil {
				continue
			}
			images[target] = bitmap
			pixels += retained
		}
		a.imageMu.Lock()
		if ticket == a.imageTicket {
			a.imageLoad = images
		}
		a.imageMu.Unlock()
		close(ready)
		a.post(func() {
			if ticket == a.imageTicket {
				a.images = images
			}
		})
	}()
}

// decodeGalleryImage returns a bitmap for a local image, scaling it down to
// limit pixels when the source is larger so that full-resolution photos still
// appear. It reports the pixels the bitmap retains.
func decodeGalleryImage(data []byte, source, limit int64) (*ui.Bitmap, int64, error) {
	if source <= limit {
		bitmap, err := ui.DecodeBitmap(data)
		if err != nil {
			return nil, 0, err
		}
		return bitmap, source, nil
	}
	src, _, err := image.Decode(bytes.NewReader(data))
	if err != nil {
		return nil, 0, err
	}
	src = orientImage(src, exifOrientation(data))
	bounds := src.Bounds()
	w, h := bounds.Dx(), bounds.Dy()
	if w <= 0 || h <= 0 {
		return nil, 0, errors.New("image has no pixels")
	}
	scale := math.Sqrt(float64(limit) / (float64(w) * float64(h)))
	dw, dh := max(1, int(float64(w)*scale)), max(1, int(float64(h)*scale))
	dst := image.NewNRGBA(image.Rect(0, 0, dw, dh))
	draw.CatmullRom.Scale(dst, dst.Bounds(), src, bounds, draw.Src, nil)
	return ui.NewBitmap(dst), int64(dw) * int64(dh), nil
}

// orientedImage presents a decoded image upright, as its EXIF orientation
// says, so scaled pictures match ui.DecodeBitmap's presentation.
type orientedImage struct {
	img image.Image
	o   int
}

func (o orientedImage) ColorModel() color.Model { return o.img.ColorModel() }

func (o orientedImage) Bounds() image.Rectangle {
	b := o.img.Bounds()
	if o.o >= 5 {
		return image.Rect(0, 0, b.Dy(), b.Dx())
	}
	return image.Rect(0, 0, b.Dx(), b.Dy())
}

func (o orientedImage) At(x, y int) color.Color {
	b := o.img.Bounds()
	w, h := b.Dx(), b.Dy()
	var sx, sy int
	switch o.o {
	case 2:
		sx, sy = w-1-x, y
	case 3:
		sx, sy = w-1-x, h-1-y
	case 4:
		sx, sy = x, h-1-y
	case 5:
		sx, sy = y, x
	case 6:
		sx, sy = y, h-1-x
	case 7:
		sx, sy = w-1-y, h-1-x
	case 8:
		sx, sy = w-1-y, x
	default:
		sx, sy = x, y
	}
	return o.img.At(b.Min.X+sx, b.Min.Y+sy)
}

func orientImage(img image.Image, orientation int) image.Image {
	if orientation <= 1 {
		return img
	}
	return orientedImage{img: img, o: orientation}
}

// exifOrientation returns the orientation of a JPEG's EXIF data, 1 to 8, or
// zero when the picture carries none.
func exifOrientation(jpg []byte) int {
	if len(jpg) < 4 || jpg[0] != 0xff || jpg[1] != 0xd8 {
		return 0
	}
	for i := 2; i+4 <= len(jpg); {
		if jpg[i] != 0xff {
			return 0
		}
		marker, size := jpg[i+1], int(binary.BigEndian.Uint16(jpg[i+2:]))
		if marker == 0xda || size < 2 || i+2+size > len(jpg) {
			return 0
		}
		seg := jpg[i+4 : i+2+size]
		if marker == 0xe1 && len(seg) > 14 && string(seg[:6]) == "Exif\x00\x00" {
			return tiffOrientation(seg[6:])
		}
		i += 2 + size
	}
	return 0
}

// tiffOrientation reads the Orientation tag of the first IFD of TIFF data.
func tiffOrientation(t []byte) int {
	if len(t) < 8 {
		return 0
	}
	var bo binary.ByteOrder
	switch string(t[:2]) {
	case "II":
		bo = binary.LittleEndian
	case "MM":
		bo = binary.BigEndian
	default:
		return 0
	}
	ifd := int(bo.Uint32(t[4:]))
	if ifd < 8 || ifd+2 > len(t) {
		return 0
	}
	n := int(bo.Uint16(t[ifd:]))
	for e := ifd + 2; e+12 <= len(t) && n > 0; e, n = e+12, n-1 {
		if bo.Uint16(t[e:]) == 0x0112 && bo.Uint16(t[e+2:]) == 3 { // Orientation, a SHORT
			if o := int(bo.Uint16(t[e+8:])); o >= 1 && o <= 8 {
				return o
			}
		}
	}
	return 0
}

// loadMarkdownImage resolves a Markdown image through the note's shared,
// bounded loader. MyGo calls it on its own goroutine, so it waits for the
// current generation and returns the decoded bitmap instead of loading files
// during frame construction.
func (a *application) loadMarkdownImage(target string) (ui.ImageSource, error) {
	a.imageMu.Lock()
	ready := a.imageReady
	a.imageMu.Unlock()
	if ready != nil {
		<-ready
	}
	a.imageMu.Lock()
	bitmap := a.imageLoad[target]
	a.imageMu.Unlock()
	if bitmap == nil {
		return nil, errors.New("image unavailable")
	}
	return bitmap, nil
}
