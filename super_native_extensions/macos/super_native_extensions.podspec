#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
# Run `pod lib lint super_native_extensions.podspec` to validate before publishing.
#
Pod::Spec.new do |s|
  s.name             = 'super_native_extensions'
  s.version          = '0.0.1'
  s.summary          = 'A new Flutter plugin project.'
  s.description      = <<-DESC
A new Flutter plugin project.
                       DESC
  s.homepage         = 'http://example.com'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'Your Company' => 'email@example.com' }

  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.dependency 'FlutterMacOS'
  s.framework = 'Carbon'

  s.platform = :osx, '10.11'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }

  # This is identitcal the script in iOS podspec
  s.script_phase = {
    :name => 'Build SuperNativeExtensions Rust library',
    :script => 'sh $PODS_TARGET_SRCROOT/../cargokit/build_pod.sh ../rust super_native_extensions',
    :execution_position=> :before_compile,
    :input_files => ['${TARGET_TEMP_DIR}/cargokit_phony'],
    :output_files => ['${TARGET_TEMP_DIR}/cargokit_phony_out'],
  }
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    # For static lib we need better control of re-exported symbols
    'EXPORTED_SYMBOLS_FILE' => '$PODS_TARGET_SRCROOT/../rust/symbols.txt',
    'OTHER_LDFLAGS' => '-lsuper_native_extensions',
    'DEAD_CODE_STRIPPING' => 'YES',
    'STRIP_INSTALLED_PRODUCT[config=Release][sdk=*][arch=*]' => "YES",
    'STRIP_STYLE[config=Release][sdk=*][arch=*]' => "non-global",
    'DEPLOYMENT_POSTPROCESSING[config=Release][sdk=*][arch=*]' => "YES",
  }

end
