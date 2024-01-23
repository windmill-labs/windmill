import {
  BodyLengthCalculator,
  ChecksumConstructor,
  Encoder,
  GetAwsChunkedEncodingStream,
  HashConstructor,
  StreamCollector,
  StreamHasher,
} from "@smithy/types";
export interface PreviouslyResolved {
  base64Encoder: Encoder;
  bodyLengthChecker: BodyLengthCalculator;
  getAwsChunkedEncodingStream: GetAwsChunkedEncodingStream;
  md5: ChecksumConstructor | HashConstructor;
  sha1: ChecksumConstructor | HashConstructor;
  sha256: ChecksumConstructor | HashConstructor;
  streamHasher: StreamHasher<any>;
  streamCollector: StreamCollector;
}
