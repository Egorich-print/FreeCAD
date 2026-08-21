#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>

class OcctKernel;

#include "freecad-kernel-occt/src/lib.rs.h"

// ShapeStatsOut, BoundsOut and FaceRangeOut are shared structs owned by the
// cxx bridge (see src/lib.rs); their C++ definitions are generated into
// freecad-kernel-occt/src/lib.rs.h and must not be redeclared here.

class OcctKernel {
public:
  OcctKernel();
  ~OcctKernel();

  std::uint64_t make_box(double dx, double dy, double dz);
  std::uint64_t make_sphere(double radius);
  std::uint64_t make_cylinder(double radius, double height);

  std::uint64_t read_step(rust::Slice<const std::uint8_t> data);
  std::uint64_t read_brep(rust::Slice<const std::uint8_t> data);
  bool write_step(std::uint64_t id, rust::Vec<std::uint8_t> &out);
  bool write_brep(std::uint64_t id, rust::Vec<std::uint8_t> &out);

  std::uint64_t fuse(std::uint64_t a, std::uint64_t b);
  std::uint64_t cut(std::uint64_t a, std::uint64_t b);
  std::uint64_t common(std::uint64_t a, std::uint64_t b);

  bool tessellate(std::uint64_t id, double linear_deflection,
                  double angular_deflection_rad, rust::Vec<float> &positions,
                  rust::Vec<float> &normals, rust::Vec<std::uint32_t> &indices,
                  rust::Vec<FaceRangeOut> &faces);

  bool shape_stats(std::uint64_t id, ShapeStatsOut &out) const;
  bool bounds(std::uint64_t id, BoundsOut &out) const;

  void destroy_shape(std::uint64_t id);
  std::size_t live_shape_count() const;
  rust::String take_error() const;

private:
  struct Impl;
  std::unique_ptr<Impl> impl_;
};

std::unique_ptr<OcctKernel> occt_kernel_new();
