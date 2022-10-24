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

<p align="center">
  <img src="https://user-images.githubusercontent.com/7232997/197459438-207ca588-43f9-4900-8049-5fe8b28ec2d4.png" width="600">
</p>

## Algorithm
- Create a spectrogram image of the audio file using a Fast Fourier Transform
- Perform a maximum filtering operation on the image
- Compare the filtered with the original image to find locations where the intensity doesn't change - these must be local peaks in the signal
- Compute hashes on nearby pairs of peaks
- These are either stored in the database, or matched against existing hashes in the database
- If many of the hashes in the sample match to a particular point in time of a particular song in the database, it is a succesful match

Original Spectrogram           |  Filtered Spectrogram
:-------------------------:|:-------------------------:
![Spectrogram of Make a Move by Lawrence](https://user-images.githubusercontent.com/7232997/197457789-843fbe1d-042b-46f0-a688-a0dbea2d7c18.png)  |  ![Maximum Filtered Spectrogram of Make a Move by Lawrence](https://user-images.githubusercontent.com/7232997/197457798-1c3ea095-b301-4b52-8dda-b6313155fda0.png)

Peak Locations
:-------------------------:
![Locations of peaks in the spectrogram](https://user-images.githubusercontent.com/7232997/197457812-29aabc17-fb23-4ad9-8534-21a000665bf6.png)

The track info and hashes are stored in a sqlite database on disk.

See https://www.ee.columbia.edu/~dpwe/papers/Wang03-shazam.pdf and https://www.cameronmacleod.com/blog/how-does-shazam-work for further explanations of the algorithm.
