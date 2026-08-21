#include "occt_shim.h"

#include <Bnd_Box.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepBndLib.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeSphere.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepTools.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <Poly_Triangulation.hxx>
#include <Standard_Failure.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopAbs_Orientation.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Shape.hxx>
#include <gp_Pnt.hxx>
#include <gp_Vec.hxx>
#include <gp_Trsf.hxx>

#include <cmath>
#include <map>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

namespace {

constexpr std::uint64_t kInvalidId = 0;

class Registry {
public:
  std::uint64_t insert(TopoDS_Shape shape) {
    const std::uint64_t id = next_id_++;
    shapes_.emplace(id, std::move(shape));
    return id;
  }

  TopoDS_Shape *find(std::uint64_t id) const {
    auto it = shapes_.find(id);
    if (it == shapes_.end()) {
      return nullptr;
    }
    return const_cast<TopoDS_Shape *>(&it->second);
  }

  void remove(std::uint64_t id) { shapes_.erase(id); }

  std::size_t size() const { return shapes_.size(); }

private:
  std::map<std::uint64_t, TopoDS_Shape> shapes_;
  std::uint64_t next_id_ = 1;
};

template <typename F> bool guard(std::string &error_slot, F &&fn) {
  try {
    fn();
    return true;
  } catch (const Standard_Failure &failure) {
    error_slot = failure.GetMessageString();
  } catch (const std::exception &exc) {
    error_slot = exc.what();
  } catch (...) {
    error_slot = "unknown OCCT failure";
  }
  return false;
}

void append_triangle(rust::Vec<float> &positions, rust::Vec<float> &normals,
                     rust::Vec<std::uint32_t> &indices, const gp_Pnt &a,
                     const gp_Pnt &b, const gp_Pnt &c) {
  const double ux = b.X() - a.X(), uy = b.Y() - a.Y(), uz = b.Z() - a.Z();
  const double vx = c.X() - a.X(), vy = c.Y() - a.Y(), vz = c.Z() - a.Z();
  double nx = uy * vz - uz * vy;
  double ny = uz * vx - ux * vz;
  double nz = ux * vy - uy * vx;
  const double len = std::sqrt(nx * nx + ny * ny + nz * nz);
  if (len > 1e-12) {
    nx /= len;
    ny /= len;
    nz /= len;
  } else {
    nx = 0.0;
    ny = 0.0;
    nz = 1.0;
  }
  const float base = static_cast<float>(positions.size() / 3);
  const float coords[3][3] = {{static_cast<float>(a.X()), static_cast<float>(a.Y()), static_cast<float>(a.Z())},
                              {static_cast<float>(b.X()), static_cast<float>(b.Y()), static_cast<float>(b.Z())},
                              {static_cast<float>(c.X()), static_cast<float>(c.Y()), static_cast<float>(c.Z())}};
  for (const auto &p : coords) {
    positions.push_back(p[0]);
    positions.push_back(p[1]);
    positions.push_back(p[2]);
    normals.push_back(static_cast<float>(nx));
    normals.push_back(static_cast<float>(ny));
    normals.push_back(static_cast<float>(nz));
  }
  indices.push_back(static_cast<std::uint32_t>(base));
  indices.push_back(static_cast<std::uint32_t>(base + 1));
  indices.push_back(static_cast<std::uint32_t>(base + 2));
}

} // namespace

struct OcctKernel::Impl {
  Registry registry;
  mutable std::string last_error;
};

OcctKernel::OcctKernel() : impl_(std::make_unique<Impl>()) {}
OcctKernel::~OcctKernel() = default;

std::unique_ptr<OcctKernel> occt_kernel_new() {
  return std::make_unique<OcctKernel>();
}

rust::String OcctKernel::take_error() const {
  rust::String out(impl_->last_error);
  impl_->last_error.clear();
  return out;
}

std::size_t OcctKernel::live_shape_count() const { return impl_->registry.size(); }

std::uint64_t OcctKernel::make_box(double dx, double dy, double dz) {
  if (dx <= 0.0 || dy <= 0.0 || dz <= 0.0) {
    impl_->last_error = "box dimensions must be positive";
    return kInvalidId;
  }
  std::uint64_t id = kInvalidId;
  bool ok = guard(impl_->last_error, [&] {
    BRepPrimAPI_MakeBox maker(dx, dy, dz);
    id = impl_->registry.insert(maker.Shape());
  });
  return ok ? id : kInvalidId;
}

std::uint64_t OcctKernel::make_sphere(double radius) {
  if (radius <= 0.0) {
    impl_->last_error = "sphere radius must be positive";
    return kInvalidId;
  }
  std::uint64_t id = kInvalidId;
  guard(impl_->last_error, [&] {
    BRepPrimAPI_MakeSphere maker(radius);
    id = impl_->registry.insert(maker.Shape());
  });
  return id;
}

std::uint64_t OcctKernel::make_cylinder(double radius, double height) {
  if (radius <= 0.0 || height <= 0.0) {
    impl_->last_error = "cylinder dimensions must be positive";
    return kInvalidId;
  }
  std::uint64_t id = kInvalidId;
  guard(impl_->last_error, [&] {
    BRepPrimAPI_MakeCylinder maker(radius, height);
    id = impl_->registry.insert(maker.Shape());
  });
  return id;
}

std::uint64_t OcctKernel::move_shape(std::uint64_t id, double dx, double dy, double dz) {
  TopoDS_Shape *shape = impl_->registry.find(id);
  if (shape == nullptr) {
    impl_->last_error = "move_shape: unknown shape id";
    return kInvalidId;
  }
  std::uint64_t new_id = kInvalidId;
  guard(impl_->last_error, [&] {
    gp_Trsf transform;
    transform.SetTranslation(gp_Vec(dx, dy, dz));
    BRepBuilderAPI_Transform maker(*shape, transform, Standard_True);
    if (maker.IsDone()) {
      new_id = impl_->registry.insert(maker.Shape());
    } else {
      impl_->last_error = "BRepBuilderAPI_Transform failed";
    }
  });
  return new_id;
}

std::uint64_t OcctKernel::read_step(rust::Slice<const std::uint8_t> data) {
  std::uint64_t id = kInvalidId;
  guard(impl_->last_error, [&] {
    std::string bytes(reinterpret_cast<const char *>(data.data()), data.size());
    std::istringstream stream(bytes);
    STEPControl_Reader reader;
    const IFSelect_ReturnStatus status = reader.ReadStream("freecad-rust.step", stream);
    if (status != IFSelect_RetDone) {
      impl_->last_error = "STEPControl_Reader::ReadStream failed";
      return;
    }
    if (reader.TransferRoots() == 0) {
      impl_->last_error = "no STEP roots transferred";
      return;
    }
    TopoDS_Shape shape = reader.OneShape();
    if (shape.IsNull()) {
      impl_->last_error = "STEP produced a null shape";
      return;
    }
    id = impl_->registry.insert(std::move(shape));
  });
  return id;
}

std::uint64_t OcctKernel::read_brep(rust::Slice<const std::uint8_t> data) {
  std::uint64_t id = kInvalidId;
  guard(impl_->last_error, [&] {
    std::string bytes(reinterpret_cast<const char *>(data.data()), data.size());
    std::istringstream stream(bytes);
    TopoDS_Shape shape;
    BRep_Builder builder;
    BRepTools::Read(shape, stream, builder);
    if (shape.IsNull()) {
      impl_->last_error = "BREP payload produced a null shape";
      return;
    }
    id = impl_->registry.insert(std::move(shape));
  });
  return id;
}

bool OcctKernel::write_step(std::uint64_t id, rust::Vec<std::uint8_t> &out) {
  TopoDS_Shape *shape = impl_->registry.find(id);
  if (shape == nullptr) {
    impl_->last_error = "write_step: unknown shape id";
    return false;
  }
  return guard(impl_->last_error, [&] {
    STEPControl_Writer writer;
    if (writer.Transfer(*shape, STEPControl_AsIs) != IFSelect_RetDone) {
      impl_->last_error = "STEPControl_Writer::Transfer failed";
      return;
    }
    std::ostringstream stream;
    if (writer.WriteStream(stream) != IFSelect_RetDone) {
      impl_->last_error = "STEPControl_Writer::WriteStream failed";
      return;
    }
    const std::string bytes = stream.str();
    out.reserve(bytes.size());
    for (char c : bytes) {
      out.push_back(static_cast<std::uint8_t>(c));
    }
  });
}

bool OcctKernel::write_brep(std::uint64_t id, rust::Vec<std::uint8_t> &out) {
  TopoDS_Shape *shape = impl_->registry.find(id);
  if (shape == nullptr) {
    impl_->last_error = "write_brep: unknown shape id";
    return false;
  }
  return guard(impl_->last_error, [&] {
    std::ostringstream stream;
    BRepTools::Write(*shape, stream);
    const std::string bytes = stream.str();
    out.reserve(bytes.size());
    for (char c : bytes) {
      out.push_back(static_cast<std::uint8_t>(c));
    }
  });
}

namespace {
std::uint64_t boolean(Registry &registry, std::string &last_error, std::uint64_t a,
                      std::uint64_t b, int kind) {
  TopoDS_Shape *lhs = registry.find(a);
  TopoDS_Shape *rhs = registry.find(b);
  if (lhs == nullptr || rhs == nullptr) {
    last_error = "boolean: unknown operand id";
    return kInvalidId;
  }
  std::uint64_t id = kInvalidId;
  guard(last_error, [&] {
    switch (kind) {
    case 0: {
      BRepAlgoAPI_Fuse op(*lhs, *rhs);
      if (!op.IsDone()) {
        last_error = "fuse failed";
        return;
      }
      id = registry.insert(op.Shape());
      break;
    }
    case 1: {
      BRepAlgoAPI_Cut op(*lhs, *rhs);
      if (!op.IsDone()) {
        last_error = "cut failed";
        return;
      }
      id = registry.insert(op.Shape());
      break;
    }
    default: {
      BRepAlgoAPI_Common op(*lhs, *rhs);
      if (!op.IsDone()) {
        last_error = "common failed";
        return;
      }
      id = registry.insert(op.Shape());
      break;
    }
    }
  });
  return id;
}
} // namespace

std::uint64_t OcctKernel::fuse(std::uint64_t a, std::uint64_t b) {
  return boolean(impl_->registry, impl_->last_error, a, b, 0);
}

std::uint64_t OcctKernel::cut(std::uint64_t a, std::uint64_t b) {
  return boolean(impl_->registry, impl_->last_error, a, b, 1);
}

std::uint64_t OcctKernel::common(std::uint64_t a, std::uint64_t b) {
  return boolean(impl_->registry, impl_->last_error, a, b, 2);
}

bool OcctKernel::tessellate(std::uint64_t id, double linear_deflection,
                            double angular_deflection_rad,
                            rust::Vec<float> &positions, rust::Vec<float> &normals,
                            rust::Vec<std::uint32_t> &indices,
                            rust::Vec<FaceRangeOut> &faces) {
  TopoDS_Shape *shape = impl_->registry.find(id);
  if (shape == nullptr) {
    impl_->last_error = "tessellate: unknown shape id";
    return false;
  }
  if (!(linear_deflection > 0.0) || !(angular_deflection_rad > 0.0)) {
    impl_->last_error = "tessellate: deflections must be positive";
    return false;
  }
  return guard(impl_->last_error, [&] {
    IMeshTools_Parameters params;
    params.Deflection = linear_deflection;
    params.Angle = angular_deflection_rad;
    params.Relative = Standard_False;
    params.InParallel = Standard_False;
    BRepMesh_IncrementalMesh mesher(*shape, params);

    std::uint32_t face_id = 0;
    for (TopExp_Explorer explorer(*shape, TopAbs_FACE); explorer.More(); explorer.Next()) {
      const TopoDS_Face face = TopoDS::Face(explorer.Current());
      TopLoc_Location location;
      const Handle(Poly_Triangulation) triangulation = BRep_Tool::Triangulation(face, location);
      if (triangulation.IsNull()) {
        continue;
      }
      const gp_Trsf transform = location.Transformation();
      const bool reversed = face.Orientation() == TopAbs_REVERSED;
      const std::uint32_t start = static_cast<std::uint32_t>(indices.size());
      const int nb_triangles = triangulation->NbTriangles();
      for (int t = 1; t <= nb_triangles; ++t) {
        int n1 = 0, n2 = 0, n3 = 0;
        triangulation->Triangle(t).Get(n1, n2, n3);
        if (reversed) {
          std::swap(n2, n3);
        }
        gp_Pnt p1 = triangulation->Node(n1).Transformed(transform);
        gp_Pnt p2 = triangulation->Node(n2).Transformed(transform);
        gp_Pnt p3 = triangulation->Node(n3).Transformed(transform);
        append_triangle(positions, normals, indices, p1, p2, p3);
      }
      const std::uint32_t count = static_cast<std::uint32_t>(indices.size()) - start;
      if (count > 0) {
        faces.push_back(FaceRangeOut{face_id, start, count});
      }
      ++face_id;
    }
    if (indices.empty()) {
      impl_->last_error = "tessellation produced no triangles";
    }
  });
}

bool OcctKernel::shape_stats(std::uint64_t id, ShapeStatsOut &out) const {
  TopoDS_Shape *shape = impl_->registry.find(id);
  if (shape == nullptr) {
    impl_->last_error = "shape_stats: unknown shape id";
    return false;
  }
  return guard(impl_->last_error, [&] {
    ShapeStatsOut stats{0, 0, 0, 0};
    TopTools_IndexedMapOfShape vertex_map, edge_map, face_map, solid_map;
    TopExp::MapShapes(*shape, TopAbs_VERTEX, vertex_map);
    TopExp::MapShapes(*shape, TopAbs_EDGE, edge_map);
    TopExp::MapShapes(*shape, TopAbs_FACE, face_map);
    TopExp::MapShapes(*shape, TopAbs_SOLID, solid_map);
    stats.vertices = static_cast<std::uint64_t>(vertex_map.Extent());
    stats.edges = static_cast<std::uint64_t>(edge_map.Extent());
    stats.faces = static_cast<std::uint64_t>(face_map.Extent());
    stats.solids = static_cast<std::uint64_t>(solid_map.Extent());
    out = stats;
  });
}

bool OcctKernel::bounds(std::uint64_t id, BoundsOut &out) const {
  TopoDS_Shape *shape = impl_->registry.find(id);
  if (shape == nullptr) {
    impl_->last_error = "bounds: unknown shape id";
    return false;
  }
  return guard(impl_->last_error, [&] {
    Bnd_Box box;
    BRepBndLib::Add(*shape, box);
    if (box.IsVoid()) {
      impl_->last_error = "bounding box is void";
      return;
    }
    Standard_Real xmin = 0, ymin = 0, zmin = 0, xmax = 0, ymax = 0, zmax = 0;
    box.Get(xmin, ymin, zmin, xmax, ymax, zmax);
    out = BoundsOut{xmin, ymin, zmin, xmax, ymax, zmax};
  });
}

void OcctKernel::destroy_shape(std::uint64_t id) { impl_->registry.remove(id); }
