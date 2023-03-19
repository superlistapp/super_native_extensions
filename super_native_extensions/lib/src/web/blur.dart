// Original source:
// https://github.com/flozz/StackBlur/blob/master/src/stackblur.js

/*
 * @copyright (c) 2010 Mario Klingemann
 *
 * Permission is hereby granted, free of charge, to any person
 * obtaining a copy of this software and associated documentation
 * files (the "Software"), to deal in the Software without
 * restriction, including without limitation the rights to use,
 * copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following
 * conditions:
 *
 * The above copyright notice and this permission notice shall be
 * included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES
 * OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
 * HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
 * WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
 * OTHER DEALINGS IN THE SOFTWARE.
 */

import 'dart:typed_data';

/// 1 Channel Stack Blur Algorithm.
void blurImageData(
    Uint8List pixels, int topX, int topY, int width, int height, int radius) {
  final div = 2 * radius + 1;
  final widthMinus1 = width - 1;
  final heightMinus1 = height - 1;
  final radiusPlus1 = radius + 1;
  final sumFactor = radiusPlus1 * (radiusPlus1 + 1) ~/ 2;

  final stackStart = _BlurStack();
  var stack = stackStart;
  late _BlurStack stackEnd;
  for (int i = 1; i < div; i++) {
    stack = stack.next = _BlurStack();
    if (i == radiusPlus1) {
      stackEnd = stack;
    }
  }
  stack.next = stackStart;
  late _BlurStack stackIn;
  late _BlurStack stackOut;

  final mulSum = _mulTable[radius];
  final shgSum = _shgTable[radius];

  int p;
  int yw = 0, yi = 0;

  for (var y = 0; y < height; y++) {
    var pr = pixels[yi], rOutSum = radiusPlus1 * pr, rSum = sumFactor * pr;

    stack = stackStart;

    for (var i = 0; i < radiusPlus1; i++) {
      stack.v = pr;
      stack = stack.next;
    }

    var rInSum = 0;
    for (var i = 1; i < radiusPlus1; i++) {
      p = yi + ((widthMinus1 < i ? widthMinus1 : i));
      rSum += (stack.v = (pr = pixels[p])) * (radiusPlus1 - i);
      rInSum += pr;
      stack = stack.next;
    }

    stackIn = stackStart;
    stackOut = stackEnd;
    for (var x = 0; x < width; x++) {
      pixels[yi] = (rSum * mulSum) >> shgSum;
      rSum -= rOutSum;
      rOutSum -= stackIn.v;
      p = (yw + ((p = x + radius + 1) < widthMinus1 ? p : widthMinus1));
      rInSum += (stackIn.v = pixels[p]);
      rSum += rInSum;
      stackIn = stackIn.next;
      rOutSum += (pr = stackOut.v);
      rInSum -= pr;
      stackOut = stackOut.next;
      yi += 1;
    }
    yw += width;
  }

  for (var x = 0; x < width; x++) {
    yi = x;
    var pr = pixels[yi], rOutSum = radiusPlus1 * pr, rSum = sumFactor * pr;

    stack = stackStart;

    for (var i = 0; i < radiusPlus1; i++) {
      stack.v = pr;
      stack = stack.next;
    }

    var rInSum = 0;
    for (var i = 1, yp = width; i <= radius; i++) {
      yi = (yp + x);
      rSum += (stack.v = (pr = pixels[yi])) * (radiusPlus1 - i);
      rInSum += pr;
      stack = stack.next;
      if (i < heightMinus1) {
        yp += width;
      }
    }

    yi = x;
    stackIn = stackStart;
    stackOut = stackEnd;
    for (var y = 0; y < height; y++) {
      p = yi;
      pixels[p] = (rSum * mulSum) >> shgSum;
      rSum -= rOutSum;
      rOutSum -= stackIn.v;
      p = (x +
          (((p = y + radiusPlus1) < heightMinus1 ? p : heightMinus1) * width));
      rSum += (rInSum += (stackIn.v = pixels[p]));
      stackIn = stackIn.next;
      rOutSum += (pr = stackOut.v);
      rInSum -= pr;
      stackOut = stackOut.next;
      yi += width;
    }
  }
}

class _BlurStack {
  int v = 0;
  late _BlurStack next;
}

const _mulTable = [
  512,
  512,
  456,
  512,
  328,
  456,
  335,
  512,
  405,
  328,
  271,
  456,
  388,
  335,
  292,
  512,
  454,
  405,
  364,
  328,
  298,
  271,
  496,
  456,
  420,
  388,
  360,
  335,
  312,
  292,
  273,
  512,
  482,
  454,
  428,
  405,
  383,
  364,
  345,
  328,
  312,
  298,
  284,
  271,
  259,
  496,
  475,
  456,
  437,
  420,
  404,
  388,
  374,
  360,
  347,
  335,
  323,
  312,
  302,
  292,
  282,
  273,
  265,
  512,
  497,
  482,
  468,
  454,
  441,
  428,
  417,
  405,
  394,
  383,
  373,
  364,
  354,
  345,
  337,
  328,
  320,
  312,
  305,
  298,
  291,
  284,
  278,
  271,
  265,
  259,
  507,
  496,
  485,
  475,
  465,
  456,
  446,
  437,
  428,
  420,
  412,
  404,
  396,
  388,
  381,
  374,
  367,
  360,
  354,
  347,
  341,
  335,
  329,
  323,
  318,
  312,
  307,
  302,
  297,
  292,
  287,
  282,
  278,
  273,
  269,
  265,
  261,
  512,
  505,
  497,
  489,
  482,
  475,
  468,
  461,
  454,
  447,
  441,
  435,
  428,
  422,
  417,
  411,
  405,
  399,
  394,
  389,
  383,
  378,
  373,
  368,
  364,
  359,
  354,
  350,
  345,
  341,
  337,
  332,
  328,
  324,
  320,
  316,
  312,
  309,
  305,
  301,
  298,
  294,
  291,
  287,
  284,
  281,
  278,
  274,
  271,
  268,
  265,
  262,
  259,
  257,
  507,
  501,
  496,
  491,
  485,
  480,
  475,
  470,
  465,
  460,
  456,
  451,
  446,
  442,
  437,
  433,
  428,
  424,
  420,
  416,
  412,
  408,
  404,
  400,
  396,
  392,
  388,
  385,
  381,
  377,
  374,
  370,
  367,
  363,
  360,
  357,
  354,
  350,
  347,
  344,
  341,
  338,
  335,
  332,
  329,
  326,
  323,
  320,
  318,
  315,
  312,
  310,
  307,
  304,
  302,
  299,
  297,
  294,
  292,
  289,
  287,
  285,
  282,
  280,
  278,
  275,
  273,
  271,
  269,
  267,
  265,
  263,
  261,
  259
];

const _shgTable = [
  9,
  11,
  12,
  13,
  13,
  14,
  14,
  15,
  15,
  15,
  15,
  16,
  16,
  16,
  16,
  17,
  17,
  17,
  17,
  17,
  17,
  17,
  18,
  18,
  18,
  18,
  18,
  18,
  18,
  18,
  18,
  19,
  19,
  19,
  19,
  19,
  19,
  19,
  19,
  19,
  19,
  19,
  19,
  19,
  19,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  20,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  21,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  22,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  23,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24,
  24
];
