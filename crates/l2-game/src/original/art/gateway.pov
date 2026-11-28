// Original Open Lords II artwork. POV-Ray 3.7; see render_gateway.py.
#version 3.7;
#include "functions.inc"
global_settings { assumed_gamma 1.0 max_trace_level 5 ambient_light rgb <.47,.55,.72> }
#default { finish { ambient .17 diffuse .78 } }

camera { location <-22,14.2,-48> look_at <8.0,9.7,14> right x*4/3 angle 52 }
light_source { <50,33,-58> color rgb <1.30,.91,.56> parallel point_at <11,3,6> }
light_source { <-65,82,-12> color rgb <.18,.28,.45> shadowless }
light_source { <30,24,88> color rgb <.47,.24,.11> shadowless }
fog { fog_type 2 distance 188 color rgb <.119,.148,.18> fog_offset -.7 fog_alt 6 }

sky_sphere {
 pigment { gradient y color_map {
     [0 rgb <.57,.334,.205>] [.055 rgb <.375,.231,.185>]
     [.14 rgb <.112,.133,.181>] [.28 rgb <.033,.062,.117>] [1 rgb <.018,.033,.075>]
 } }
 pigment { bozo turbulence .56 octaves 5 lambda 2.1 omega .58
     color_map { [0 rgbt <.18,.15,.16,1>] [.39 rgbt <.18,.13,.145,1>]
     [.52 rgbt <.125,.104,.133,.75>] [.62 rgbt <.092,.096,.147,.45>]
     [.74 rgbt <.058,.078,.134,.21>] [1 rgbt <.044,.063,.107,.31>] }
     scale <.26,.08,.23> translate <.10,.021,.32>
 }
}

#declare S = seed(12485);
#declare StoneFinish = finish { ambient .13 diffuse .80 roughness .09 specular .10 };
#macro Stone(B)
    #local A = B*(.82+.34*rand(S));
    texture {
        pigment { granite color_map { [0 rgb <.108,.102,.094>*A]
            [.27 rgb <.235,.224,.199>*A] [.57 rgb <.306,.291,.261>*A]
            [.84 rgb <.385,.369,.333>*A] [1 rgb <.443,.428,.390>*A] }
            scale .35 }
        normal { granite .26 scale .16 }
        finish { StoneFinish }
    }
#end
#declare Mortar = texture { pigment { rgb <.141,.133,.123> } normal { granite .31 scale .18 } finish { StoneFinish } };
#declare Iron = texture { pigment { rgb <.047,.044,.038> } finish { ambient .15 diffuse .57 specular .39 roughness .14 } };
#declare Timber = texture {
    pigment { wood turbulence .09 color_map { [0 rgb <.058,.032,.021>] [.63 rgb <.126,.078,.047>] [1 rgb <.072,.047,.031>] } scale <.069,4,.14> }
    normal { wood .48 scale <.071,2,.19> } finish { StoneFinish }
};
#declare Interior = texture { pigment { rgb <.023,.018,.015> } finish { ambient .08 diffuse .24 } };
#declare Warm = texture { pigment { rgb <.75,.252,.055> } finish { emission rgb <.38,.12,.022> diffuse .5 } };
#declare Slate = texture {
    pigment { granite color_map { [0 rgb <.029,.033,.044>] [.45 rgb <.072,.077,.092>] [1 rgb <.163,.154,.146>] } scale .28 }
    normal { granite .24 scale .22 } finish { ambient .12 diffuse .72 specular .23 roughness .085 }
};
#declare Ground = texture {
    pigment { bozo turbulence .47 color_map { [0 rgb <.049,.058,.034>]
        [.30 rgb <.071,.084,.047>] [.49 rgb <.115,.123,.063>]
        [.61 rgb <.148,.132,.076>] [.77 rgb <.200,.173,.102>] [1 rgb <.24,.211,.145>] } scale 1.7 }
    normal { granite .51 scale .29 } finish { StoneFinish }
};
#declare Bedrock = texture {
    pigment { bozo turbulence .5 color_map { [0 rgb <.055,.055,.050>]
        [.35 rgb <.144,.135,.113>] [.57 rgb <.25,.223,.180>] [.78 rgb <.333,.306,.263>]
        [1 rgb <.369,.350,.318>] } scale <1.65,.35,1.65> }
    normal { granite .36 scale .22 } finish { StoneFinish }
};
#declare Road = texture {
    pigment { granite color_map { [0 rgb <.089,.072,.049>] [.49 rgb <.22,.17,.110>] [1 rgb <.39,.32,.22>] } scale .55 }
    normal { granite .47 scale .16 } finish { StoneFinish }
};
#declare Water = texture {
    pigment { rgb <.031,.056,.071> }
    normal { bumps .115 scale <1.15,1,.41> }
    finish { ambient .10 diffuse .27 reflection { .10,.45 fresnel on } specular .50 roughness .05 }
};
plane { y, .02 texture { Water } interior { ior 1.333 } }

#declare IslandMask = function(X,Z) {
    min(1,max(0,(1-pow((X-16)/23,2)-pow((Z-16)/select(Z-16,25.4,32),2))*4))
};
#declare Island = height_field {
    function 450,530 { IslandMask(x*58-12,54-y*78)*(.47+.093*f_noise3d(x*16,.8,y*16)+.032*f_noise3d(x*44,1,y*48)) }
    smooth scale <58,8.4,78> translate <-12,-1.95,-24>
    texture { slope y texture_map { [0 Bedrock] [.66 Bedrock] [.79 Ground] [1 Ground] } }
};
object { Island }
#declare LeftBank = height_field {
    function 350,420 { min(1,max(0,(-.17+x*.3+pow(abs(y-.52)*2,1.5)*.28)*(.93+.6*f_noise3d(x*17,1,y*18)))) }
    smooth scale <63,26,150> rotate y*164 translate <-46,-1.55,101>
    texture { Ground }
};
object { LeftBank }
#declare Foreground = height_field {
    function 430,440 { min(1,max(0,(.849-(1-y)+abs(x-.57)*.28+.05*exp(-pow((x-.509)*13,2)))*17))
        *(.29+.124*f_noise3d(x*14,0,y*13)+.025*f_noise3d(x*42,1,y*40)) }
    smooth scale <115,11,95> translate <-58,-1.1,-94>
    texture { slope y texture_map { [0 Bedrock] [.58 Bedrock] [.74 Ground] [1 Ground] } }
};
object { Foreground }

#macro Ridge(Z,W,H,Col,Seed)
    #local R = seed(Seed);
    #local K = rand(R)*10;
    height_field {
        function 375,140 { max(0, (sin(y*pi)*sin(x*pi))*(.08+pow(f_noise3d(x*9,K,y*3),1.6)*.91
            +pow(f_noise3d(x*22,K+.8,y*10),2)*.24)) }
        smooth scale <W,H,W*.31> translate <-W*.56,-1.1,Z>
        texture { pigment { granite color_map { [0 rgb Col*.7] [1 rgb Col*1.21] } scale 17 }
                  normal { bumps .32 scale 3.3 } finish { ambient .62 diffuse .53 } }
    }
#end
Ridge(178,288,49,<.195,.206,.234>,72)
Ridge(114,247,37,<.132,.158,.181>,828)
Ridge(76,171,25,<.10,.125,.133>,79)

#macro Wall(L,H,D,B)
    union {
        box { <-L/2+.05,-.05,-D/2+.048>,<L/2-.05,H-.057,D/2-.048> texture { Mortar } }
        #local Y = 0;
        #while (Y < H-.02)
            #local Rise = min(.45+.17*rand(S),H-Y);
            #local Left = -L/2;
            #while (Left<L/2-.024)
                #local Right = min(L/2,Left+.71+.59*rand(S));
                #if (Right-Left>.052)
                    box { <Left+.012,Y+.012,-D/2-.031*rand(S)>,
                          <Right-.012,Y+Rise-.012,D/2+.031*rand(S)> Stone(B) }
                #end
                #local Left = Right;
            #end
            #local Y = Y+Rise;
        #end
    }
#end
#macro Arch(W,H,D)
    union {
        box { <-W/2,-.06,-D/2>,<W/2,H,D/2> }
        cylinder { <0,H,-D/2>,<0,H,D/2>,W/2 }
    }
#end
#macro ArcStone(R0,R1,A1,A2,B0,B1)
    prism { linear_sweep linear_spline B0,B1,7,
        <R0*cos(A1),R0*sin(A1)>,<R1*cos(A1),R1*sin(A1)>,
        <R1*cos((A1+A2)/2),R1*sin((A1+A2)/2)>,<R1*cos(A2),R1*sin(A2)>,
        <R0*cos(A2),R0*sin(A2)>,<R0*cos((A1+A2)/2),R0*sin((A1+A2)/2)>,
        <R0*cos(A1),R0*sin(A1)>
    }
#end
#macro ArchTrim(W,H,F,T,B)
    #local I = 0;
    #while (I<11)
        #local A = I*pi/11+.007;
        #local E = (I+1)*pi/11-.007;
        object { ArcStone(W/2,W/2+T,A,E,-.14,.08) rotate x*-90 translate <0,H,F> Stone(B) }
        #local I = I+1;
    #end
    #local Y = 0;
    #while (Y<H)
        #local Top = min(H,Y+.45);
        #if (Top>Y+.027)
            box { <W/2,Y+.014,F-.08>,<W/2+T,Top-.013,F+.14> Stone(B) }
            box { <-W/2-T,Y+.014,F-.08>,<-W/2,Top-.013,F+.14> Stone(B) }
        #end
        #local Y = Top;
    #end
#end
#macro Slit(W,H,F,Lit)
    object { Arch(W,H,.05) translate z*F texture { Interior } }
    ArchTrim(W,H,F-.015,.11,1.02)
    #if (Lit)
        object { Arch(W*.52,H-.14,.015) translate <0,.06,F-.031> texture { Warm } }
        box { <-.03,.04,F-.051>,<.03,H+.1,F-.025> texture { Iron } }
    #end
#end
#macro Parapet(L,H,D)
    object { Wall(L,.34,D+.17,1.08) translate y*H }
    #local N = max(2,int(L/1.5));
    #local I = 0;
    #while (I<N)
        #local X = -L/2+(I+.5)*L/N;
        object { Wall(.75,.69,D,1.05) translate <X,H+.32,0> }
        box { <X-.416,H+.99,-D*.58>,<X+.416,H+1.105,D*.58> Stone(1.1) }
        #local I = I+1;
    #end
#end

#macro Tower(X,Y,Z,R,H,Roof)
  union {
    cylinder { <0,-.01,0>,<0,H,0>,R-.086 texture { Mortar } }
    #local Row = 0;
    #local B = 0;
    #local N = int(R*6.6);
    #while (B<H-.02)
        #local Top = min(B+.46,H);
        #local Edge = R + .17*max(0,1-B/.8) + .19*max(0,1-abs(B-H+.52)*2.1);
        #local I = 0;
        #while (I<N)
            #local A = (I+mod(Row,2)*.52)*2*pi/N;
            object { ArcStone(R-.16,Edge+.04*rand(S),A+.008,A+2*pi/N-.008,B+.016,Top-.015) Stone(1) }
            #local I = I+1;
        #end
        #local B = Top;
        #local Row = Row+1;
    #end
    cylinder { <0,H-.16,0>,<0,H+.05,0>,R+.19 Stone(1.11) }
    #if (Roof)
        #local RH = R*1.55;
        cone { <0,H-.05,0>,R+.27,<0,H+RH,0>,.07 texture { Slate } }
        #local I = 0;
        #while (I<10)
            #local F = I/10;
            #local G = (I+1)/10;
            cone { <0,H+RH*F,0>,(R+.33)*(1-F),<0,H+RH*G,0>,(R+.2)*(1-G) texture { Slate } }
            #local I = I+1;
        #end
        cylinder { <0,H+RH-.05,0>,<0,H+RH+.48,0>,.034 texture { Iron } }
        sphere { <0,H+RH+.48,0>,.085 texture { Iron } }
    #else
        #local N = int(R*4);
        #local I = 0;
        #while (I<N)
            #local A = I*2*pi/N;
            object { ArcStone(R-.39,R+.17,A,A+pi/N,H,H+.72) Stone(1.04) }
            object { ArcStone(R-.43,R+.23,A-.026,A+pi/N+.026,H+.72,H+.84) Stone(1.13) }
            #local I = I+1;
        #end
    #end
    #for (Dir,-45,95,35)
        union {
            Slit(.20,.94,-R-.055,1-min(1,mod(Dir+45,3)))
            translate y*(H-2.29) rotate y*Dir
        }
    #end
    translate <X,Y,Z>
  }
#end

// Front curtain, flanking drum towers, gate, and higher rear castle.
#declare Base = 2.52;
object { Wall(8.9,5.25,1.52,.93) translate <7.35,Base,0> }
object { Wall(8.9,5.25,1.52,.93) translate <22.65,Base,0> }
object { Wall(21,5.04,1.55,.94) rotate y*90 translate <27,Base,10.5> }
union { Parapet(20,7.56,1.45) rotate y*90 translate <27,0,11> }
object { Wall(20,4.51,1.45,.94) rotate y*90 translate <3,Base,10> }
union { Parapet(19,7.03,1.38) rotate y*90 translate <3,0,10> }
object { Wall(24,5.9,1.46,.92) translate <15,Base,21> }
union { Parapet(24,8.42,1.39) translate <15,0,21> }
union { Parapet(9.5,7.77,1.45) translate <8.25,0,0> }
union { Parapet(9.5,7.77,1.45) translate <22.65,0,0> }
Tower(3,2.24,0,2.08,8.86,0)
Tower(27,2.35,.12,2.35,10.6,0)
Tower(3,2.49,20,2.24,11.28,1)
Tower(27,2.42,21,2.49,11.5,1)
Tower(12.27,2.20,-1.24,1.58,8.28,0)
Tower(18.03,2.20,-1.24,1.58,8.28,0)
union {
    difference {
        object { Wall(6.5,7.25,3.6,1.02) }
        object { Arch(2.85,2,4.7) }
    }
    ArchTrim(2.85,2,-1.88,.40,1.21)
    object { Arch(2.80,2,0.12) translate z*-.98 texture { Timber } }
    #for (I,-6,6)
        cylinder { <I*.205,.03,-1.69>,<I*.205,2+sqrt(1.40*1.40-pow(I*.205,2)),-1.69>,.023 texture { Iron } }
    #end
    #for (I,0,7)
        #local H = I*.38+.14;
        #local A = sqrt(max(.01,1.4*1.4-pow(max(0,H-2),2)));
        cylinder { <-A,H,-1.69>,<A,H,-1.69>,.024 texture { Iron } }
    #end
    union { Slit(.46,.66,-1.848,1) translate <-.05,4.25,0> }
    Parapet(6.23,7.18,3.4)
    translate <15.15,2.49,-.45>
}
#declare KeepX = 18.4;
#declare KeepZ = 13.7;
#declare KeepW = 9.95;
#declare KeepD = 8.65;
#declare KeepH = 15.5;
union {
    box { <-KeepW/2,-.4,-KeepD/2>,<KeepW/2,KeepH,KeepD/2> texture { Mortar } }
    #for (Sign,-1,1,2)
        union { Wall(KeepW,KeepH,.52,.99) translate z*(Sign*KeepD/2) }
        union { Wall(KeepD,KeepH,.52,.91) rotate y*90 translate x*(Sign*KeepW/2) }
        union { Parapet(KeepW+.7,KeepH,.68) translate z*(Sign*KeepD/2) }
        union { Parapet(KeepD+.6,KeepH,.68) rotate y*90 translate x*(Sign*KeepW/2) }
    #end
    #for (T,1,3)
        #local Hi = 3+T*2.8;
        box { <-KeepW/2-.32,Hi-.10,-KeepD/2-.33>,<KeepW/2+.32,Hi,KeepD/2+.33> Stone(1.03) }
    #end
    #for (I,-1,1)
        union { Slit(.44,1.05,-KeepD/2-.30,1-abs(I)) translate <I*2.39,12.2,0> }
        union { Slit(.28,.85,-KeepD/2-.30,max(0,I)) translate <I*2.25,8.31,0> }
        union { Slit(.29,1.12,-KeepW/2-.30,max(0,I)) rotate y*90 translate <0,11.37,I*2.29> }
    #end
    translate <KeepX,Base,KeepZ>
}
Tower(22.43,Base,17.16,1.45,17.2,1)
Tower(13.76,Base,10.05,1.23,16.08,0)

#declare Cloth = texture {
    pigment { gradient y color_map { [0 rgb <.107,.012,.020>] [.46 rgb <.21,.029,.032>] [1 rgb <.36,.048,.050>] } }
    normal { waves .17 scale .23 } finish { ambient .16 diffuse .76 }
};
#declare Pennant = mesh {
    #for (I,0,18)
        #local X = I*.142;
        #local N = (I+1)*.142;
        #local H = .51*(1-X/3.3);
        #local J = .51*(1-N/3.3);
        #local P = .14*sin(5*X)+.06*sin(13*X);
        #local Q = .14*sin(5*N)+.06*sin(13*N);
        triangle { <X,H,P>,<X,-H,P>,<N,J,Q> }
        triangle { <X,-H,P>,<N,-J,Q>,<N,J,Q> }
    #end
    texture { Cloth } double_illuminate
};
#macro Flag(X,Y,Z,A)
    cylinder { <X,Y,Z>,<X,Y+2.35,Z>,.045 texture { Iron } }
    sphere { <X,Y+2.38,Z>,.077 texture { Iron } }
    object { Pennant rotate y*A translate <X,Y+1.68,Z> }
#end
Flag(17.7,18.47,11.5,-12)
Flag(27,13.73,.12,-12)
Flag(3,10.89,-.1,-12)

#macro Torch(X,Y,Z)
    cylinder { <X,Y-.29,Z+.13>,<X,Y,Z>,.063 texture { Iron } }
    sphere { 0,.092 scale <.69,1.51,.69> translate <X,Y+.08,Z> texture { Warm } }
    light_source { <X,Y+.10,Z-.10> color rgb <.92,.34,.095> fade_distance 1.55 fade_power 2 }
#end
Torch(13.43,4.75,-2.66)
Torch(16.87,4.75,-2.66)

// Three river spans; timber entrance to dressed stone arcades, seen obliquely.
union {
    difference {
        object { Wall(17.7,2.96,3.37,1.02) }
        #for (A,-1,1)
            object { Arch(3.68,.89,4.4) translate <A*4.85,0,0> }
        #end
        translate y*-.23
    }
    #for (A,-1,1)
        union { ArchTrim(3.68,.89,-1.71,.31,1.12) translate <A*4.85,-.23,0> }
        union { ArchTrim(3.68,.89,-1.71,.31,1.02) rotate y*180 translate <A*4.85,-.23,0> }
    #end
    box { <-8.89,2.72,-1.77>,<8.89,2.91,1.77> texture { Road } }
    union { Wall(17.8,.51,.24,1.08) translate <0,2.87,-1.62> }
    union { Wall(17.8,.51,.24,1.08) translate <0,2.87,1.62> }
    #for (A,-4,4)
        box { <A*2.14-.22,2.78,-1.89>,<A*2.14+.22,3.64,-1.43> Stone(1.05) }
        box { <A*2.14-.22,2.78,1.43>,<A*2.14+.22,3.64,1.89> Stone(1.05) }
    #end
    rotate y*-34 translate <7.75,0,-7.55>
}

// A trace-conforming road joins the bridge to the near forest.
#macro RoadPoint(T,Side)
    #local X = .34-T*.75+2.8*sin(T*.10)+Side*(1.23+.11*sin(1.24*T));
    #local Z = -12.52-T;
    #local V = trace(Foreground,<X,18,Z>,-y);
    (V+y*(.06+.05*max(0,1-T)))
#end
mesh {
    #for (I,0,97)
        #local T = I*.34;
        #local A = RoadPoint(T,-1);
        #local B = RoadPoint(T,1);
        #local C = RoadPoint(T+.34,-1);
        #local D = RoadPoint(T+.34,1);
        triangle { A,B,C }
        triangle { B,C,D }
    #end
    texture { Road }
}

#macro TreeTemplate(K)
    #local R = seed(K);
    union {
        cone { <0,.03,0>,.107,<0,3.46,0>,.032 texture { Timber } }
        #for (I,0,12)
            #local H = .65+I*.222;
            #local W = (3.67-H)*(.21+.081*rand(R));
            #local T = .76+.3*rand(R);
            #local Shade = .67+.58*rand(R);
            mesh {
                #local N = 14;
                #local J = 0;
                #local A = I*1.821;
                #local V = <W*cos(A),H,W*sin(A)>;
                #while (J<N)
                    #local B = A+(J+1)*2*pi/N;
                    #local Bump = .74+.28*rand(R);
                    #local X = .53+.44*mod(J,2);
                    #local U = <W*Bump*X*cos(B),H+.19*(.35-rand(R)),W*Bump*X*sin(B)>;
                    triangle { <.059,H+T,.041>,V,U }
                    triangle { <0,H+.13,0>,U,V }
                    #local V = U;
                    #local J = J+1;
                #end
                texture { pigment { granite color_map { [0 rgb <.012,.027,.023>*Shade]
                    [.56 rgb <.033,.056,.037>*Shade] [1 rgb <.096,.113,.062>*Shade] } scale .16 }
                    normal { bumps .45 scale .19 } finish { ambient .19 diffuse .77 } }
                double_illuminate
            }
        #end
    }
#end
#declare FirA = object { TreeTemplate(17) };
#declare FirB = object { TreeTemplate(712) };
#declare FirC = object { TreeTemplate(1138) };
#macro Fir(X,Y,Z,T)
    #local I = int(rand(S)*3);
    #local Model = FirA;
    #if (I=1)
        #local Model = FirB;
    #elseif (I=2)
        #local Model = FirC;
    #end
    object { Model scale <T*(.78+.29*rand(S)),T,T*(.78+.29*rand(S))> rotate y*(rand(S)*360) translate <X,Y,Z> }
#end
#for (I,0,225)
    #local X = -26+rand(S)*72;
    #local Z = 26+rand(S)*66;
    #local N = <0,0,0>;
    #local V = trace(Island,<X,21,Z>,-y,N);
    #if (V.y>.35 & N.y>.59 & (X<4|X>27|Z>23))
        Fir(X,V.y-.035,Z,.46+.77*rand(S))
    #end
    #local V = trace(LeftBank,<X-12,37,Z>,-y,N);
    #if (V.y>.28 & N.y>.65)
        Fir(X-12,V.y-.035,Z,.42+.71*rand(S))
    #end
#end
#for (I,0,240)
    #local X = -30+rand(S)*72;
    #local Z = -42+rand(S)*70;
    #local N = <0,0,0>;
    #local V = trace(Foreground,<X,16,Z>,-y,N);
    #if (V.y>.22 & N.y>.45 & (X<-9 | X>10))
        Fir(X,V.y-.08,Z,.51+1.38*pow(rand(S),1.66))
    #end
    #local V = trace(Island,<X,15,Z>,-y,N);
    #if (V.y>.22 & N.y>.55 & (X<1 | X>28 | Z<-1.2 & (X<6|X>17)))
        Fir(X,V.y-.08,Z,.44+.86*pow(rand(S),1.32))
    #end
#end

// Small eroded rocks at the water line break up the profile of the headland.
#for (I,0,120)
    #local X = -2+rand(S)*42;
    #local Z = -18+rand(S)*47;
    #local N = <0,0,0>;
    #local V = trace(Island,<X,16,Z>,-y,N);
    #if (V.y>.1 & V.y<2.5 & (X<3 | X>27 | Z<-1.5))
        #local R = .19+pow(rand(S),2)*.86;
        sphere { 0,1 scale <R,R*(.31+.39*rand(S)),R> rotate z*(rand(S)*60-30) translate V texture { Bedrock } }
    #end
#end
