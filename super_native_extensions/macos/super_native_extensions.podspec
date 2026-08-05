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
  s.source_files     = 'super_native_extensions/Sources/super_native_extensions/**/*.{h,m}'
  s.public_header_files = 'super_native_extensions/Sources/super_native_extensions/include/**/*.h'
  s.dependency 'FlutterMacOS'
  s.framework = 'Carbon'

  s.platform = :osx, '10.11'

  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'DEAD_CODE_STRIPPING' => 'YES',
    'STRIP_INSTALLED_PRODUCT[config=Release][sdk=*][arch=*]' => "YES",
    'STRIP_STYLE[config=Release][sdk=*][arch=*]' => "non-global",
    'DEPLOYMENT_POSTPROCESSING[config=Release][sdk=*][arch=*]' => "YES",
  }

end
