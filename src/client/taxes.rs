// Copyright Materialize, Inc. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE file at the
// root of this repository, or online at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use codes_iso_3166::part_1::CountryCode;
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};

/// The subset of [`TaxId`] used in create and update requests.
#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct TaxIdRequest<'a> {
    /// The type of the tax ID.
    #[serde(rename = "type")]
    pub type_: TaxIdType,
    /// The value of the tax ID.
    pub value: &'a str,
    /// The country of the tax ID.
    pub country: CountryCode,
}

/// Tax ID details to display on an invoice.
#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct TaxId {
    /// The type of the tax ID.
    #[serde(rename = "type")]
    pub type_: TaxIdType,
    /// The value of the tax ID.
    pub value: String,
    /// The country of the tax ID.
    pub country: CountryCode,
}

/// The type of a [`TaxId`].
///
/// See: <https://docs.withorb.com/docs/orb-docs/api-reference/schemas/customer-tax-id>
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Hash, Deserialize_enum_str, Serialize_enum_str)]
#[serde(rename_all = "snake_case")]
pub enum TaxIdType {
    /// United Arab Emirates Tax Registration Number.
    AeTrn,
    /// Australian Business Number.
    AuAbn,
    /// Australian Taxation Office Reference Number.
    AuArn,
    /// Bulgaria Unified Identification Code.
    BgUic,
    /// Brazilian CNPJ number.
    BrCnpj,
    /// Brazilian CPF number.
    BrCpf,
    /// Canadian BN.
    CaBn,
    /// Canadian GST/HST number.
    CaGstHst,
    /// Canadian PST number (British Columbia).
    CaPstBc,
    /// Canadian PST number (Manitoba).
    CaPstMb,
    /// Canadian PST number (Saskatchewan).
    CaPstSk,
    /// Canadian QST number (Québec).
    CaQst,
    /// Switzerland VAT number.
    ChVat,
    /// Chilean TIN.
    ClTin,
    /// Spanish NIF number (previously Spanish CIF number).
    EsCif,
    /// European One Stop Shop VAT number for non-Union scheme.
    EuOssVat,
    /// European VAT number.
    EuVat,
    /// United Kingdom VAT number.
    GbVat,
    /// Georgian VAT.
    GeVat,
    /// Hong Kong BR number.
    HkBr,
    /// Hungary tax number.
    HuTin,
    /// Indonesian NPWP number.
    IdNpwp,
    /// Israel VAT.
    IlVat,
    /// Indian GST number.
    InGst,
    /// Icelandic VAT.
    IsVat,
    /// Japanese Corporate Number.
    JpCn,
    /// Japanese Registered Foreign Businesses' Registration Number.
    JpRn,
    /// Japanese Tax Registration Number.
    JpTrn,
    /// Korean BRN.
    KrBrn,
    /// Liechtensteinian UID number.
    LiUid,
    /// Mexican RFC number.
    MxRfc,
    /// Malaysian FRP number.
    MyFrp,
    /// Malaysian ITN.
    MyItn,
    /// Malaysian SST number.
    MySst,
    /// Norwegian VAT number.
    NoVat,
    /// New Zealand GST number
    NzGst,
    /// Russian INN.
    RuInn,
    /// Russian KPP.
    RuKpp,
    /// Saudi Arabia VAT.
    SaVat,
    /// Singaporean GST.
    SgGst,
    /// Singaporean UEN.
    SgUen,
    /// Slovenia tax number.
    SiTin,
    /// Thai VAT.
    ThVat,
    /// Taiwanese VAT.
    TwVat,
    /// Ukrainian VAT.
    UaVat,
    /// United States EIN.
    UsEin,
    /// South African VAT number.
    ZaVat,
    /// Other.
    #[serde(other)]
    Other(String),
}
