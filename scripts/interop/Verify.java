import javax.xml.parsers.*;
import javax.xml.crypto.dsig.*;
import javax.xml.crypto.dsig.dom.*;
import java.nio.file.*;
import java.security.cert.*;
import java.util.*;
import org.w3c.dom.*;
public class Verify {
 public static void main(String[] args)throws Exception {
  var factory=DocumentBuilderFactory.newInstance();factory.setNamespaceAware(true);
  var doc=factory.newDocumentBuilder().parse(args[0]+"/signature.xml");
  var cert=(X509Certificate)CertificateFactory.getInstance("X.509").generateCertificate(Files.newInputStream(Path.of(args[0]+"/cert.der")));
  var context=new DOMValidateContext(cert.getPublicKey(),doc.getDocumentElement());
  context.setProperty("org.jcp.xml.dsig.secureValidation",false); // Fixture verifies the legacy SHA-1 protocol MPass requests.
  var nodes=doc.getElementsByTagName("*");for(int i=0;i<nodes.getLength();i++){var e=(Element)nodes.item(i);if(e.hasAttribute("Id"))context.setIdAttributeNS(e,null,"Id");}
  var signature=XMLSignatureFactory.getInstance("DOM").unmarshalXMLSignature(context);
  boolean okay=signature.validate(context);
  Files.write(Path.of(args[0]+"/java-canonical.xml"),signature.getSignedInfo().getCanonicalizedData().readAllBytes());
  System.out.println("Signature value: "+signature.getSignatureValue().validate(context));
  System.out.println("Independent Java XMLDSig validation: "+okay);
  for(Object item:signature.getSignedInfo().getReferences()){var ref=(Reference)item;System.out.println("Reference "+ref.getURI()+": "+ref.validate(context));}
  var issuer=doc.getElementsByTagNameNS("http://www.w3.org/2000/09/xmldsig#","X509IssuerName").item(0).getTextContent();
  boolean issuerMatches=new javax.security.auth.x500.X500Principal(issuer).equals(cert.getIssuerX500Principal());
  System.out.println("XAdES certificate issuer matches: "+issuerMatches);
  var root=doc.getDocumentElement();
  var objects=new ArrayList<Element>();
  for(var child=root.getFirstChild();child!=null;child=child.getNextSibling())if(child instanceof Element e && "Object".equals(e.getLocalName()))objects.add(e);
  boolean objectOrder=objects.size()==2 && objects.get(0).getElementsByTagNameNS("http://uri.etsi.org/01903/v1.3.2#","QualifyingProperties").getLength()==1 && objects.get(1).getAttribute("Id").startsWith("FileObject-");
  System.out.println("MPass object layout matches: "+objectOrder);
  boolean legacyCanonicalization=signature.getSignedInfo().getCanonicalizationMethod().getAlgorithm().equals("http://www.w3.org/TR/2001/REC-xml-c14n-20010315#WithComments");
  for(Object item:signature.getSignedInfo().getReferences())legacyCanonicalization &= ((Reference)item).getTransforms().isEmpty();
  System.out.println("MoldSign canonicalization profile matches: "+legacyCanonicalization);
  if(!okay || !issuerMatches || !objectOrder || !legacyCanonicalization)System.exit(1);
 }
}
