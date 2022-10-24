# atlas

Implementation of Shazam Entertainment's [algorithm](https://www.ee.columbia.edu/~dpwe/papers/Wang03-shazam.pdf) for audio matching. 

## Installation
Requires the [rust compiler](https://www.rust-lang.org/tools/install)

```
git clone https://github.com/owenbrooks/atlas.git
cd atlas
```

## Usage

Add a directory (`tracks/` in this case) of .wav files to the database (must be 16 bit samples, mono or stereo): 

`cargo run --release -- -i tracks/ add`

Run matching: 

`cargo run --release -- -i sample.wav match`

## Algorithm
- Create a spectrogram image of the audio file using a Fast Fourier Transform
- Perform a maximum filtering operation on the image
- Compare the filtered with the original image to find locations where the intensity doesn't change - these must be local peaks in the signal
- Compute hashes on nearby pairs of peaks
- These are either stored in the database, or matched against existing hashes in the database
- If many of the hashes in the sample match to a particular point in time of a particular song in the database, it is a succesful match

See https://www.ee.columbia.edu/~dpwe/papers/Wang03-shazam.pdf and https://www.cameronmacleod.com/blog/how-does-shazam-work for further explanations of the algorithm.