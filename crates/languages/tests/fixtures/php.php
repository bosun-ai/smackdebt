<?php
function outer($a, $b, ...$more) {
 $inner = function($x) { if ($x > 0) { return $x; } return 0; };
 if ($a > $b) { return $inner($a); } return $b;
}
