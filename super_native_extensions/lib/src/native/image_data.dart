import '../image_data.dart';
import '../util.dart';

extension ImageDataExt on ImageData {
  dynamic serialize() => {
        'width': width,
        'height': height,
        'bytesPerRow': bytesPerRow,
        'data': data,
        'devicePixelRatio': devicePixelRatio,
      };
}

extension TargettedImageDataExt on TargetedImageData {
  dynamic serialize() => {
        'imageData': imageData.serialize(),
        'rect': rect.serialize(),
      };
}
