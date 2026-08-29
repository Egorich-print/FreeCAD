// SPDX-License-Identifier: LGPL-2.1-or-later

/***************************************************************************
 *   Copyright (c) 2011 Juergen Riegel <FreeCAD@juergen-riegel.net>        *
 *                                                                         *
 *   This file is part of the FreeCAD CAx development system.              *
 *                                                                         *
 *   This library is free software; you can redistribute it and/or         *
 *   modify it under the terms of the GNU Library General Public           *
 *   License as published by the Free Software Foundation; either          *
 *   version 2 of the License, or (at your option) any later version.      *
 *                                                                         *
 *   This library  is distributed in the hope that it will be useful,      *
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of        *
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
 *   GNU Library General Public License for more details.                  *
 *                                                                         *
 *   You should have received a copy of the GNU Library General Public     *
 *   License along with this library; see the file COPYING.LIB. If not,    *
 *   write to the Free Software Foundation, Inc., 59 Temple Place,         *
 *   Suite 330, Boston, MA  02111-1307, USA                                *
 *                                                                         *
 ***************************************************************************/

#include <QAction>
#include <QDoubleValidator>
#include <QFontMetrics>
#include <QListWidget>
#include <QMessageBox>

#include <Base/Interpreter.h>
#include <App/Document.h>
#include <App/DocumentObject.h>
#include <Gui/Selection/Selection.h>
#include <Gui/Tools.h>
#include <Gui/ViewProvider.h>
#include <Gui/Inventor/Draggers/SoLinearDragger.h>
#include <Gui/Inventor/Draggers/SoRotationDragger.h>
#include <Gui/Utilities.h>
#include <Mod/PartDesign/App/FeatureChamfer.h>
#include <Mod/Part/App/Geometry.h>
#include <Mod/Part/App/GizmoHelper.h>

#include <Precision.hxx>
#include <TopoDS.hxx>
#include <BRep_Tool.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <GeomAPI_ProjectPointOnSurf.hxx>
#include <Base/Converter.h>

#include "ui_TaskChamferParameters.h"
#include "TaskChamferParameters.h"


using namespace PartDesignGui;
using namespace Gui;

/* TRANSLATOR PartDesignGui::TaskChamferParameters */

TaskChamferParameters::TaskChamferParameters(ViewProviderDressUp* DressUpView, QWidget* parent)
    : TaskDressUpParameters(DressUpView, true, true, parent)
    , ui(new Ui_TaskChamferParameters)
{
    // we need a separate container widget to add all controls to
    proxy = new QWidget(this);
    ui->setupUi(proxy);
    this->groupLayout()->addWidget(proxy);

    PartDesign::Chamfer* pcChamfer = DressUpView->getObject<PartDesign::Chamfer>();

    setUpUI(pcChamfer);

    bool useAllEdges = pcChamfer->UseAllEdges.getValue();
    ui->checkBoxUseAllEdges->setChecked(useAllEdges);
    ui->buttonRefSel->setEnabled(!useAllEdges);
    ui->listWidgetReferences->setEnabled(!useAllEdges);
    QMetaObject::invokeMethod(ui->chamferSize, "setFocus", Qt::QueuedConnection);

    std::vector<std::string> strings = pcChamfer->Base.getSubValues();
    for (const auto& string : strings) {
        ui->listWidgetReferences->addItem(QString::fromStdString(string));
    }

    QMetaObject::connectSlotsByName(this);

    // clang-format off
    connect(ui->chamferType, qOverload<int>(&QComboBox::currentIndexChanged),
            this, &TaskChamferParameters::onTypeChanged);
    connect(ui->chamferSize, qOverload<double>(&Gui::QuantitySpinBox::valueChanged),
            this, &TaskChamferParameters::onSizeChanged);
    connect(ui->chamferSize2, qOverload<double>(&Gui::QuantitySpinBox::valueChanged),
            this, &TaskChamferParameters::onSize2Changed);
    connect(ui->chamferAngle, qOverload<double>(&Gui::QuantitySpinBox::valueChanged),
            this, &TaskChamferParameters::onAngleChanged);
    connect(ui->flipDirection, &QCheckBox::toggled,
            this, &TaskChamferParameters::onFlipDirection);
    connect(ui->buttonRefSel, &QToolButton::toggled,
            this, &TaskChamferParameters::onButtonRefSel);
    connect(ui->checkBoxUseAllEdges, &QCheckBox::toggled,
            this, &TaskChamferParameters::onCheckBoxUseAllEdgesToggled);

    // Create context menu
    createDeleteAction(ui->listWidgetReferences);
    connect(deleteAction, &QAction::triggered,
            this, &TaskChamferParameters::onRefDeleted);

    createAddAllEdgesAction(ui->listWidgetReferences);
    connect(addAllEdgesAction, &QAction::triggered,
            this, &TaskChamferParameters::onAddAllEdges);

    connect(ui->listWidgetReferences, &QListWidget::currentItemChanged,
            this, &TaskChamferParameters::setSelection);
    connect(ui->listWidgetReferences, &QListWidget::itemClicked,
            this, &TaskChamferParameters::setSelection);
    connect(ui->listWidgetReferences, &QListWidget::itemDoubleClicked,
            this, &TaskChamferParameters::doubleClicked);
    // clang-format on

    setupGizmos(DressUpView);

    if (strings.size() == 0) {
        setSelectionMode(refSel);
    }
    else {
        hideOnError();
    }
}

void TaskChamferParameters::setUpUI(PartDesign::Chamfer* pcChamfer)
{
    const int index = pcChamfer->ChamferType.getValue();
    ui->chamferType->setCurrentIndex(index);

    ui->flipDirection->setEnabled(index != 0);  // Enable if type is not "Equal distance"
    ui->flipDirection->setChecked(pcChamfer->FlipDirection.getValue());

    ui->chamferSize->setUnit(Base::Unit::Length);
    ui->chamferSize->setMinimum(0.0);
    ui->chamferSize->setMaximum(10000.0);
    ui->chamferSize->setSingleStep(0.1);  // Fine control for radius
    ui->chamferSize->setValue(pcChamfer->Size.getValue());
    ui->chamferSize->setToolTip(QT_TR_NOOP("Radius of the chamfer (0.1 to 10000 mm)"));
    ui->chamferSize->bind(pcChamfer->Size);
    ui->chamferSize->selectNumber();
    // Add validation via property bounds + clamp on valueChanged
    connect(ui->chamferSize, qOverload<double>(&Gui::QuantitySpinBox::valueChanged), this, [this]() {
        if (auto chamfer = getObject<PartDesign::Chamfer>()) {
            double value = chamfer->Size.getValue();
            if (value < 0.0) {
                chamfer->Size.setValue(0.0);
            } else if (value > 10000.0) {
                chamfer->Size.setValue(10000.0);
            }
        }
    });

    ui->chamferSize2->setUnit(Base::Unit::Length);
    ui->chamferSize2->setMinimum(0.0);
    ui->chamferSize2->setMaximum(10000.0);
    ui->chamferSize2->setSingleStep(0.1);
    ui->chamferSize2->setValue(pcChamfer->Size2.getValue());
    ui->chamferSize2->setToolTip(QT_TR_NOOP("Second radius of the chamfer (0.1 to 10000 mm)"));
    ui->chamferSize2->bind(pcChamfer->Size2);
    // Add validation via property bounds + clamp on valueChanged
    connect(ui->chamferSize2, qOverload<double>(&Gui::QuantitySpinBox::valueChanged), this, [this]() {
        if (auto chamfer = getObject<PartDesign::Chamfer>()) {
            double value = chamfer->Size2.getValue();
            if (value < 0.0) {
                chamfer->Size2.setValue(0.0);
            } else if (value > 10000.0) {
                chamfer->Size2.setValue(10000.0);
            }
        }
    });

    ui->chamferAngle->setUnit(Base::Unit::Angle);
    ui->chamferAngle->setMinimum(pcChamfer->Angle.getMinimum());
    ui->chamferAngle->setMaximum(pcChamfer->Angle.getMaximum());
    ui->chamferAngle->setSingleStep(1.0);
    ui->chamferAngle->setValue(pcChamfer->Angle.getValue());
    ui->chamferAngle->setToolTip(QT_TR_NOOP("Chamfer angle (0-90° typical)"));
    ui->chamferAngle->bind(pcChamfer->Angle);
    // Add validation via property bounds + clamp on valueChanged
    double angleMin = pcChamfer->Angle.getMinimum();
    double angleMax = pcChamfer->Angle.getMaximum();
    connect(ui->chamferAngle, qOverload<double>(&Gui::QuantitySpinBox::valueChanged), this, [this, angleMin, angleMax]() {
        if (auto chamfer = getObject<PartDesign::Chamfer>()) {
            double value = chamfer->Angle.getValue();
            if (value < angleMin) {
                chamfer->Angle.setValue(angleMin);
            } else if (value > angleMax) {
                chamfer->Angle.setValue(angleMax);
            }
        }
    });
    ui->flipDirection->setToolTip(QT_TR_NOOP("Swap two distances / flip chamfer side"));

    ui->stackedWidget->setFixedHeight(ui->chamferSize2->sizeHint().height());

    QFontMetrics fm(ui->typeLabel->font());
    int minWidth = Gui::QtTools::horizontalAdvance(fm, ui->typeLabel->text());
    minWidth = std::max<int>(minWidth, Gui::QtTools::horizontalAdvance(fm, ui->sizeLabel->text()));
    minWidth = std::max<int>(minWidth, Gui::QtTools::horizontalAdvance(fm, ui->size2Label->text()));
    minWidth = std::max<int>(minWidth, Gui::QtTools::horizontalAdvance(fm, ui->angleLabel->text()));
    minWidth = minWidth + 5;  // spacing
    ui->typeLabel->setMinimumWidth(minWidth);
    ui->sizeLabel->setMinimumWidth(minWidth);
    ui->size2Label->setMinimumWidth(minWidth);
    ui->angleLabel->setMinimumWidth(minWidth);
}

void TaskChamferParameters::onSelectionChanged(const Gui::SelectionChanges& msg)
{
    // executed when the user selected something in the CAD object
    // adds/deletes the selection accordingly

    if (msg.Type == Gui::SelectionChanges::AddSelection) {
        if (selectionMode == refSel) {
            referenceSelected(msg, ui->listWidgetReferences);
            // keep gizmo in sync when new edge added (Fusion-like)
            setGizmoPositions();
        }
    }
    else if (msg.Type == Gui::SelectionChanges::ClrSelection) {
        setGizmoPositions();
    }
}

void TaskChamferParameters::onCheckBoxUseAllEdgesToggled(bool checked)
{
    if (auto chamfer = getObject<PartDesign::Chamfer>()) {
        if (checked) {
            setSelectionMode(none);
        }

        ui->buttonRefSel->setEnabled(!checked);
        ui->listWidgetReferences->setEnabled(!checked);
        chamfer->UseAllEdges.setValue(checked);
        chamfer->recomputeFeature();
    }
}

void TaskChamferParameters::setButtons(const selectionModes mode)
{
    ui->buttonRefSel->setChecked(mode == refSel);
    ui->buttonRefSel->setText(mode == refSel ? stopSelectionLabel() : startSelectionLabel());
}

void TaskChamferParameters::onRefDeleted()
{
    TaskDressUpParameters::deleteRef(ui->listWidgetReferences);
    setGizmoPositions();
}

void TaskChamferParameters::onAddAllEdges()
{
    TaskDressUpParameters::addAllEdges(ui->listWidgetReferences);
}

void TaskChamferParameters::onTypeChanged(int index)
{
    if (auto chamfer = getObject<PartDesign::Chamfer>()) {
        setSelectionMode(none);
        chamfer->ChamferType.setValue(index);
        ui->stackedWidget->setCurrentIndex(index);
        ui->flipDirection->setEnabled(index != 0);  // Enable if type is not "Equal distance"
        chamfer->recomputeFeature();
        // hide the chamfer if there was a computation error
        hideOnError();
    }
}

void TaskChamferParameters::onSizeChanged(double len)
{
    if (auto chamfer = getObject<PartDesign::Chamfer>()) {
        setSelectionMode(none);
        setupTransaction();
        chamfer->Size.setValue(len);
        chamfer->recomputeFeature();
        // hide the chamfer if there was a computation error
        hideOnError();
    }
}

void TaskChamferParameters::onSize2Changed(double len)
{
    if (auto chamfer = getObject<PartDesign::Chamfer>()) {
        setSelectionMode(none);
        setupTransaction();
        chamfer->Size2.setValue(len);
        chamfer->recomputeFeature();
        // hide the chamfer if there was a computation error
        hideOnError();
    }
}

void TaskChamferParameters::onAngleChanged(double angle)
{
    if (auto chamfer = getObject<PartDesign::Chamfer>()) {
        setSelectionMode(none);
        setupTransaction();
        chamfer->Angle.setValue(angle);
        chamfer->recomputeFeature();
        // hide the chamfer if there was a computation error
        hideOnError();
    }
}

void TaskChamferParameters::onFlipDirection(bool flip)
{
    if (auto chamfer = getObject<PartDesign::Chamfer>()) {
        setSelectionMode(none);
        setupTransaction();
        chamfer->FlipDirection.setValue(flip);
        chamfer->recomputeFeature();
        // hide the chamfer if there was a computation error
        hideOnError();

        setGizmoPositions();
    }
}

int TaskChamferParameters::getType() const
{
    return ui->chamferType->currentIndex();
}

double TaskChamferParameters::getSize() const
{
    return ui->chamferSize->value().getValue();
}

double TaskChamferParameters::getSize2() const
{
    return ui->chamferSize2->value().getValue();
}

double TaskChamferParameters::getAngle() const
{
    return ui->chamferAngle->value().getValue();
}

bool TaskChamferParameters::getFlipDirection() const
{
    return ui->flipDirection->isChecked();
}

TaskChamferParameters::~TaskChamferParameters()
{
    try {
        Gui::Selection().clearSelection();
        Gui::Selection().rmvSelectionGate();
    }
    catch (const Py::Exception&) {
        Base::PyException e;  // extract the Python error text
        e.reportException();
    }
}

void TaskChamferParameters::changeEvent(QEvent* e)
{
    TaskBox::changeEvent(e);
    if (e->type() == QEvent::LanguageChange) {
        ui->retranslateUi(proxy);
    }
}

void TaskChamferParameters::apply()
{
    auto chamfer = getObject<PartDesign::Chamfer>();

    const int chamfertype = chamfer->ChamferType.getValue();

    switch (chamfertype) {

        case 0:  // "Equal distance"
            ui->chamferSize->apply();
            break;
        case 1:  // "Two distances"
            ui->chamferSize->apply();
            ui->chamferSize2->apply();
            break;
        case 2:  // "Distance and Angle"
            ui->chamferSize->apply();
            ui->chamferAngle->apply();
            break;
    }

    // Alert user if he created an empty feature
    if (ui->listWidgetReferences->count() == 0) {
        Base::Console().warning(tr("Empty chamfer created!\n").toStdString().c_str());
    }
}

void TaskChamferParameters::setupGizmos(ViewProviderDressUp* vp)
{
    if (!GizmoContainer::isEnabled()) {
        return;
    }

    // M5: Fusion-parity — distinct handles, correct binding, multFactor correction
    distanceGizmo = new Gui::LinearGizmo(ui->chamferSize);
    distanceGizmo->setDraggerStyle(Gui::LinearDraggerStyle::Arrow); // Arrow for distance1

    secondDistanceGizmo = new Gui::LinearGizmo(ui->chamferSize2);
    secondDistanceGizmo->setDraggerStyle(Gui::LinearDraggerStyle::Sphere); // Sphere for distance2

    angleGizmo = new Gui::RotationGizmo(ui->chamferAngle);

    connect(ui->chamferType, qOverload<int>(&QComboBox::currentIndexChanged), [this](int index) {
        auto type = static_cast<Part::ChamferType>(index);

        switch (type) {
            case Part::ChamferType::equalDistance:
                // Two handles for one value — like Fillet, helps to grasp from either face
                secondDistanceGizmo->setVisibility(true);
                angleGizmo->setVisibility(false);
                secondDistanceGizmo->setProperty(ui->chamferSize); // Both bind to Size
                distanceGizmo->setVisibility(true);
                break;
            case Part::ChamferType::twoDistances:
                secondDistanceGizmo->setVisibility(true);
                angleGizmo->setVisibility(false);
                secondDistanceGizmo->setProperty(ui->chamferSize2); // Bind to Size2
                distanceGizmo->setVisibility(true);
                break;
            case Part::ChamferType::distanceAngle:
                secondDistanceGizmo->setVisibility(false);
                angleGizmo->setVisibility(true);
                distanceGizmo->setVisibility(true);
                break;
        }
        setGizmoPositions();
    });

    // keep gizmo in sync when user picks reference via list
    connect(
        ui->listWidgetReferences,
        &QListWidget::currentRowChanged,
        this,
        [this](int) { setGizmoPositions(); }
    );

    gizmoContainer = GizmoContainer::create({distanceGizmo, secondDistanceGizmo, angleGizmo}, vp);

    setGizmoPositions();

    // trigger initial visibility/binding without double emission side-effects
    ui->chamferType->currentIndexChanged(ui->chamferType->currentIndex());
    showDraggerHints();
}

void TaskChamferParameters::setGizmoPositions()
{
    if (!gizmoContainer) {
        return;
    }

    auto chamfer = getObject<PartDesign::Chamfer>();
    if (!chamfer) {
        gizmoContainer->visible = false;
        return;
    }

    // M5: do not hide gizmo on compute error — keep it for live correction,
    // just dim/hint; user can drag back to valid size
    if (chamfer->isError()) {
        // keep visible so user can drag back; actual error is shown in task panel
        gizmoContainer->visible = true;
    }

    PartDesign::TopoShape baseShape = chamfer->getBaseTopoShape(true);
    auto shapes = chamfer->getContinuousEdges(baseShape);

    if (shapes.empty()) {
        gizmoContainer->visible = false;
        return;
    }
    gizmoContainer->visible = true;

    // Fusion-like: pick edge under list selection if any, else first continuous edge
    Part::TopoShape edge;
    int selRow = ui->listWidgetReferences->currentRow();
    if (selRow >= 0 && selRow < static_cast<int>(shapes.size())) {
        // try to resolve selected subname to actual edge; fallback to shapes[selRow]
        QString selText = ui->listWidgetReferences->item(selRow)->text();
        try {
            edge = baseShape.getSubTopoShape(selText.toStdString().c_str());
            if (edge.isNull()) {
                edge = shapes[static_cast<size_t>(selRow)];
            }
        }
        catch (...) {
            edge = shapes[static_cast<size_t>(selRow)];
        }
    }
    else {
        edge = shapes[0];
    }

    auto [face1, face2] = getAdjacentFacesFromEdge(edge, baseShape);

    DraggerPlacementProps props1 = getDraggerPlacementFromEdgeAndFace(edge, face1);
    DraggerPlacementProps props2 = getDraggerPlacementFromEdgeAndFace(edge, face2);
    if (ui->flipDirection->isChecked()) {
        std::swap(props1, props2);
    }

    distanceGizmo->Gizmo::setDraggerPlacement(props1.position, props1.dir);
    secondDistanceGizmo->Gizmo::setDraggerPlacement(props2.position, props2.dir);

    // M5: multFactor correction — visual dragger length = actual value
    // Like Fillet: correction = 1/tan(angle/2) where angle is between face normals
    double angle = props1.dir.GetAngle(props2.dir);
    double correction = 1.0;
    if (angle > Precision::Confusion() && angle < M_PI - Precision::Confusion()) {
        correction = 1.0 / std::tan(angle / 2.0);
    }
    distanceGizmo->setMultFactor(correction);
    secondDistanceGizmo->setMultFactor(correction);

    // angle handle sits below linear; for chamfer it shares edge midpoint
    angleGizmo->placeBelowLinearGizmo(distanceGizmo);
    Base::Vector3d cross = -props1.dir.Cross(props2.dir);
    if (cross.Length() > Precision::Confusion()) {
        angleGizmo->getDraggerContainer()->setArcNormalDirection(Base::convertTo<SbVec3f>(cross));
    }
    // Only show the gizmo if the chamfer type is set to distance and angle
    angleGizmo->setVisibility(getType() == 2);
    // ensure distance handles visibility matches type (redundant safety)
    if (getType() == 2) {
        secondDistanceGizmo->setVisibility(false);
    }
    else {
        secondDistanceGizmo->setVisibility(true);
    }
}

//**************************************************************************
//**************************************************************************
// TaskDialog
//++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

TaskDlgChamferParameters::TaskDlgChamferParameters(ViewProviderChamfer* DressUpView)
    : TaskDlgDressUpParameters(DressUpView)
{
    parameter = new TaskChamferParameters(DressUpView);

    Content.push_back(parameter);
    Content.push_back(preview);
}

TaskDlgChamferParameters::~TaskDlgChamferParameters() = default;

//==== calls from the TaskView ===============================================================

bool TaskDlgChamferParameters::accept()
{
    auto obj = getObject();
    if (!obj->isError()) {
        getViewObject()->showPreviousFeature(false);
    }

    parameter->apply();

    return TaskDlgDressUpParameters::accept();
}

#include "moc_TaskChamferParameters.cpp"
