#include "recording/audio/audio_capture_reader_factory.h"

#include "recording/audio/pulse_audio_capture_reader.h"
#include "recording/audio/wasapi_audio_capture_reader.h"

namespace markshot::recording {

bool recordingAudioCaptureAvailable()
{
#if defined(HAVE_PULSE_RECORDING) || defined(_WIN32)
    return true;
#else
    return false;
#endif
}

QString recordingAudioUnavailableText()
{
    return recordingAudioCaptureAvailable()
        ? QString()
        : QStringLiteral("Audio recording is not available in this build.");
}

std::unique_ptr<AudioCaptureReader> createPlatformAudioCaptureReader()
{
#ifdef _WIN32
    return std::make_unique<WasapiAudioCaptureReader>();
#elif defined(HAVE_PULSE_RECORDING)
    return std::make_unique<PulseAudioCaptureReader>();
#else
    return nullptr;
#endif
}

}  // namespace markshot::recording
